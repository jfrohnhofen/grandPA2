#[cfg(windows)]
pub mod windows_capture {
    use windows::core::Interface;
    use windows::Graphics::Capture::{Direct3D11CaptureFramePool, GraphicsCaptureItem};
    use windows::Graphics::DirectX::DirectXPixelFormat;
    use windows::Win32::Foundation::{BOOL, HWND, LPARAM, POINT, RECT};
    use windows::Win32::Graphics::Direct3D::{D3D_DRIVER_TYPE_HARDWARE, D3D_FEATURE_LEVEL_11_0};
    use windows::Win32::Graphics::Direct3D11::{
        D3D11CreateDevice, ID3D11Device, ID3D11DeviceContext, ID3D11Texture2D, D3D11_CPU_ACCESS_READ,
        D3D11_CREATE_DEVICE_BGRA_SUPPORT, D3D11_MAPPED_SUBRESOURCE, D3D11_MAP_READ, D3D11_SDK_VERSION,
        D3D11_TEXTURE2D_DESC, D3D11_USAGE_STAGING,
    };
    use windows::Win32::Graphics::Dwm::{DwmGetWindowAttribute, DWMWA_EXTENDED_FRAME_BOUNDS};
    use windows::Win32::Graphics::Dxgi::IDXGIDevice;
    use windows::Win32::Graphics::Gdi::ClientToScreen;
    use windows::Win32::System::WinRT::Direct3D11::{
        CreateDirect3D11DeviceFromDXGIDevice, IDirect3DDxgiInterfaceAccess,
    };
    use windows::Win32::System::WinRT::Graphics::Capture::IGraphicsCaptureItemInterop;
    use windows::Win32::UI::WindowsAndMessaging::{
        EnumWindows, GetClientRect, GetWindowRect, GetWindowTextW, IsWindowVisible,
    };

    use tokio::sync::mpsc;
    /// Start background LED monitor loop
    pub fn start(led_tx: mpsc::Sender<Vec<(usize, bool)>>) {
        use crate::keymap::MASTER_KEY_MAP;

        unsafe {
            use windows::Win32::UI::HiDpi::{
                SetProcessDpiAwarenessContext, DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2,
            };
            let _ = SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
        }

        std::thread::spawn(move || {
            println!("[LED]      Starting LED monitor...");
            let monitored_keys: Vec<_> = MASTER_KEY_MAP
                .iter()
                .enumerate()
                .filter_map(|(idx, key)| key.norm_coords.map(|coords| (idx, coords)))
                .collect();

            let mut last_led_states: Vec<Option<bool>> = vec![None; monitored_keys.len()];

            // Find grandMA2 window
            let hwnd = loop {
                let mut result: Option<(HWND, String)> = None;
                unsafe {
                    let _ = EnumWindows(Some(enum_windows_callback), LPARAM(&mut result as *mut _ as isize));
                }
                if let Some((hwnd, title)) = result {
                    println!("[LED]      Found grandMA2 window '{}'", title);
                    break hwnd;
                }
                std::thread::sleep(std::time::Duration::from_secs(1));
            };

            unsafe {
                let mut d3d_device: Option<ID3D11Device> = None;
                let mut d3d_context: Option<ID3D11DeviceContext> = None;
                let feature_levels = [D3D_FEATURE_LEVEL_11_0];
                let mut obtained_feature_level = D3D_FEATURE_LEVEL_11_0;

                let setup = || -> windows::core::Result<_> {
                    D3D11CreateDevice(
                        None,
                        D3D_DRIVER_TYPE_HARDWARE,
                        None,
                        D3D11_CREATE_DEVICE_BGRA_SUPPORT,
                        Some(&feature_levels),
                        D3D11_SDK_VERSION,
                        Some(&mut d3d_device),
                        Some(&mut obtained_feature_level),
                        Some(&mut d3d_context),
                    )?;
                    let d3d_device = d3d_device.unwrap();
                    let d3d_context = d3d_context.unwrap();
                    let dxgi_device: IDXGIDevice = d3d_device.cast()?;
                    let inspectable = CreateDirect3D11DeviceFromDXGIDevice(&dxgi_device)?;
                    let winrt_device: windows::Graphics::DirectX::Direct3D11::IDirect3DDevice = inspectable.cast()?;

                    let interop = windows::core::factory::<GraphicsCaptureItem, IGraphicsCaptureItemInterop>()?;

                    let create_pipeline = |interop: &IGraphicsCaptureItemInterop,
                                           winrt_device: &windows::Graphics::DirectX::Direct3D11::IDirect3DDevice|
                     -> windows::core::Result<_> {
                        let item: GraphicsCaptureItem = interop.CreateForWindow(hwnd)?;
                        let item_size = item.Size()?;
                        let frame_pool = Direct3D11CaptureFramePool::CreateFreeThreaded(
                            winrt_device,
                            DirectXPixelFormat::B8G8R8A8UIntNormalized,
                            2,
                            item_size,
                        )?;
                        let session = frame_pool.CreateCaptureSession(&item)?;
                        let _ = session.SetIsCursorCaptureEnabled(false);
                        session.StartCapture()?;
                        Ok((item, frame_pool, session))
                    };

                    let (item, frame_pool, session) = create_pipeline(&interop, &winrt_device)?;

                    Ok((interop, frame_pool, session, d3d_device, d3d_context, winrt_device, item, create_pipeline))
                };

                let (
                    interop,
                    mut frame_pool,
                    mut session,
                    d3d_device,
                    d3d_context,
                    winrt_device,
                    mut item,
                    create_pipeline,
                ) = match setup() {
                    Ok(res) => res,
                    Err(e) => {
                        println!("[LED]      Setting up screen capture failed: {:?}", e);
                        return;
                    }
                };

                println!("[LED]      Screen capture started. Monitoring {} LEDs...", monitored_keys.len());

                let mut cached_staging_texture: Option<ID3D11Texture2D> = None;
                let mut cached_width = 0;
                let mut cached_height = 0;
                let mut last_item_size = item.Size().unwrap_or_default();

                loop {
                    std::thread::sleep(std::time::Duration::from_millis(16));

                    if let Ok(current_size) = item.Size() {
                        if current_size.Width != last_item_size.Width || current_size.Height != last_item_size.Height {
                            if current_size.Width > 0 && current_size.Height > 0 {
                                println!("[LED]      Window resized, recreating capture pipeline...");
                                let _ = session.Close();
                                let _ = frame_pool.Close();

                                if let Ok((new_item, new_pool, new_session)) = create_pipeline(&interop, &winrt_device)
                                {
                                    item = new_item;
                                    frame_pool = new_pool;
                                    session = new_session;
                                    last_item_size = current_size;
                                    cached_staging_texture = None;
                                    cached_width = 0;
                                    cached_height = 0;
                                }
                            }
                        }
                    }

                    let (geom, mapped, staging_texture, width, height) = match (|| -> windows::core::Result<_> {
                        let geom = get_window_geometry(hwnd)?;
                        let frame = frame_pool.TryGetNextFrame()?;
                        let surface = frame.Surface()?;
                        let access: IDirect3DDxgiInterfaceAccess = surface.cast()?;
                        let src_texture: ID3D11Texture2D = access.GetInterface()?;

                        let mut desc = D3D11_TEXTURE2D_DESC::default();
                        src_texture.GetDesc(&mut desc);

                        if desc.Width == 0 || desc.Height == 0 {
                            return Err(windows::core::Error::empty());
                        }

                        if cached_width != desc.Width
                            || cached_height != desc.Height
                            || cached_staging_texture.is_none()
                        {
                            let staging_desc = D3D11_TEXTURE2D_DESC {
                                Width: desc.Width,
                                Height: desc.Height,
                                MipLevels: 1,
                                ArraySize: 1,
                                Format: desc.Format,
                                SampleDesc: windows::Win32::Graphics::Dxgi::Common::DXGI_SAMPLE_DESC {
                                    Count: 1,
                                    Quality: 0,
                                },
                                Usage: D3D11_USAGE_STAGING,
                                BindFlags: 0,
                                CPUAccessFlags: D3D11_CPU_ACCESS_READ.0 as u32,
                                MiscFlags: 0,
                            };

                            let mut new_staging: Option<ID3D11Texture2D> = None;
                            d3d_device.CreateTexture2D(&staging_desc, None, Some(&mut new_staging))?;
                            cached_staging_texture = new_staging;
                            cached_width = desc.Width;
                            cached_height = desc.Height;
                        }

                        let staging_texture = cached_staging_texture.as_ref().unwrap().clone();

                        d3d_context.CopyResource(&staging_texture, &src_texture);

                        let mut mapped = D3D11_MAPPED_SUBRESOURCE::default();
                        d3d_context.Map(&staging_texture, 0, D3D11_MAP_READ, 0, Some(&mut mapped))?;

                        Ok((geom, mapped, staging_texture, desc.Width, desc.Height))
                    })() {
                        Ok(res) => res,
                        Err(_) => continue,
                    };

                    let mapped_ptr = mapped.pData as *const u8;
                    let pitch = mapped.RowPitch as usize;

                    let total_bytes = pitch * (height as usize);
                    let bgra_pixels = std::slice::from_raw_parts(mapped_ptr, total_bytes).to_vec();

                    d3d_context.Unmap(&staging_texture, 0);

                    // Dynamic target calculation relative to trimmed UI canvas
                    let (trimmed_w, trimmed_h) = calculate_target_ar_crop(geom.client_w, geom.client_h);

                    let mut updates = Vec::new();

                    // Using the original DWM client offsets
                    let final_offset_x = geom.client_offset_x as u32;
                    let final_offset_y = geom.client_offset_y as u32;

                    for (idx, &(orig_idx, coords)) in monitored_keys.iter().enumerate() {
                        let target_x = final_offset_x as i32 + (coords.0 * trimmed_w as f64) as i32;
                        let target_y = final_offset_y as i32 + (coords.1 * trimmed_h as f64) as i32;
                        let width_i32 = width as i32;
                        let height_i32 = height as i32;

                        // 7x7 pixel region yellow check
                        let is_yellow = (-3..=3).flat_map(|dy| (-3..=3).map(move |dx| (dx, dy))).any(|(dx, dy)| {
                            let px = target_x + dx;
                            let py = target_y + dy;

                            if px >= 0 && px < width_i32 && py >= 0 && py < height_i32 {
                                let px_idx = (py as usize) * pitch + (px as usize) * 4;
                                if px_idx + 2 < bgra_pixels.len() {
                                    let b = bgra_pixels[px_idx];
                                    let g = bgra_pixels[px_idx + 1];
                                    let r = bgra_pixels[px_idx + 2];
                                    return r >= 180 && g >= 180 && b <= 90;
                                }
                            }
                            false
                        });

                        if last_led_states[idx] != Some(is_yellow) {
                            last_led_states[idx] = Some(is_yellow);
                            updates.push((orig_idx, is_yellow));
                        }
                    }

                    if !updates.is_empty() {
                        let _ = led_tx.blocking_send(updates);
                    }
                }
            }
        });
    }

    unsafe extern "system" fn enum_windows_callback(hwnd: HWND, lparam: LPARAM) -> BOOL {
        let result = &mut *(lparam.0 as *mut Option<(HWND, String)>);
        if IsWindowVisible(hwnd).as_bool() {
            let mut text: [u16; 512] = [0; 512];
            let len = GetWindowTextW(hwnd, &mut text);
            if len > 0 {
                let title = String::from_utf16_lossy(&text[..len as usize]);
                if title.to_lowercase().contains("grandma2") && title.to_lowercase().contains("onpc") {
                    *result = Some((hwnd, title));
                    return BOOL(0); // Stop enumeration
                }
            }
        }
        BOOL(1) // Continue enumeration
    }

    #[derive(Debug, Clone, Copy)]
    struct WindowGeometry {
        pub client_w: u32,
        pub client_h: u32,
        pub client_offset_x: i32,
        pub client_offset_y: i32,
    }

    fn get_window_geometry(hwnd: HWND) -> windows::core::Result<WindowGeometry> {
        unsafe {
            let mut window_rect = RECT::default();
            GetWindowRect(hwnd, &mut window_rect)?;

            let mut frame_rect = RECT::default();
            let _ = DwmGetWindowAttribute(
                hwnd,
                DWMWA_EXTENDED_FRAME_BOUNDS,
                &mut frame_rect as *mut _ as *mut _,
                std::mem::size_of::<RECT>() as u32,
            );

            let mut client_rect = RECT::default();
            GetClientRect(hwnd, &mut client_rect)?;
            let client_w = (client_rect.right - client_rect.left) as u32;
            let client_h = (client_rect.bottom - client_rect.top) as u32;

            let mut client_pt = POINT { x: 0, y: 0 };
            let _ = ClientToScreen(hwnd, &mut client_pt);

            let client_offset_x = (client_pt.x - frame_rect.left).max(0);
            let client_offset_y = (client_pt.y - frame_rect.top).max(0);

            Ok(WindowGeometry { client_w, client_h, client_offset_x, client_offset_y })
        }
    }

    const TARGET_ASPECT_RATIO: f64 = 1.6898059996399053;

    fn calculate_target_ar_crop(client_w: u32, client_h: u32) -> (u32, u32) {
        let current_ar = client_w as f64 / client_h as f64;
        if current_ar > TARGET_ASPECT_RATIO {
            let trimmed_w = (client_h as f64 * TARGET_ASPECT_RATIO).round() as u32;
            (trimmed_w.min(client_w), client_h)
        } else {
            let trimmed_h = (client_w as f64 / TARGET_ASPECT_RATIO).round() as u32;
            (client_w, trimmed_h.min(client_h))
        }
    }
}

#[cfg(not(windows))]
pub mod windows_capture {
    use tokio::sync::mpsc;

    #[allow(dead_code)]
    pub fn start(_led_tx: mpsc::Sender<Vec<(usize, bool)>>) {
        println!("[LED]      Screen capture not supported on this platform");
    }
}

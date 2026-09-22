use clap::Parser;
use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tokio::sync::mpsc;
use tokio::time::sleep;
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};

#[derive(Parser, Debug, Clone)]
#[command(name = "grandPA2 Host Application", version = "0.2.0")]
pub struct CliArgs {
    /// grandMA2 WebRemote target in format [user[:pass]@]host[:port] (e.g. grandpa2:secret@127.0.0.1:80)
    #[arg(default_value = "grandpa2:secret@127.0.0.1:80")]
    pub target: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GrandMA2Config {
    pub username: String,
    pub password: String,
    pub host: String,
    pub port: u16,
}

impl GrandMA2Config {
    pub fn parse(input: &str) -> Self {
        let mut user = "grandpa2".to_string();
        let mut pass = "secret".to_string();
        let mut host = "127.0.0.1".to_string();
        let mut port = 80u16;

        let mut remainder = input.trim();

        // Extract user[:pass]@
        if let Some(at_idx) = remainder.find('@') {
            let auth_str = &remainder[..at_idx];
            remainder = &remainder[at_idx + 1..];

            if let Some(colon_idx) = auth_str.find(':') {
                user = auth_str[..colon_idx].to_string();
                pass = auth_str[colon_idx + 1..].to_string();
            } else if !auth_str.is_empty() {
                user = auth_str.to_string();
            }
        }

        // Extract host[:port]
        if let Some(colon_idx) = remainder.rfind(':') {
            if let Ok(parsed_port) = remainder[colon_idx + 1..].parse::<u16>() {
                port = parsed_port;
                host = remainder[..colon_idx].to_string();
            } else {
                host = remainder.to_string();
            }
        } else if !remainder.is_empty() {
            host = remainder.to_string();
        }

        Self { username: user, password: pass, host, port }
    }

    pub fn url(&self) -> String {
        format!("ws://{}:{}/", self.host, self.port)
    }
}

/// Commands that can be sent to grandMA2 onPC
#[derive(Debug, Clone)]
pub enum GrandMA2Command {
    /// Send a WebRemote HardKey command using CANBUS code
    HardKey { code: u8, pressed: bool },
    /// Send relative attribute encoder turn (e.g. "Dimmer", "Pan", "Tilt") with delta count
    Encoder { attribute: String, delta: i32, fine: bool },
    /// Set Executor Fader level (executor 1..4, level 0..127)
    ExecFader { exec_num: u8, level: u8 },
    /// Executor main button press/release (executor 1..4, pressed)
    ExecButton { exec_num: u8, pressed: bool },
    /// Set Grand Master level (0..127)
    GrandMaster { level: u8 },
    /// Send an arbitrary command line string to GrandMA2
    Command { cmd: String },
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct GrandMA2LoginPayload {
    request_type: String,
    username: String,
    password: String,
    session: u64,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct GrandMA2CmdPayload {
    request_type: String,
    command: String,
    session: u64,
}

#[derive(Deserialize, Debug, Default)]
#[serde(default, rename_all = "camelCase")]
struct GrandMA2PresetTypesResponse {
    pre: Vec<GrandMA2PresetGroup>,
}

#[derive(Deserialize, Debug, Default)]
#[serde(default)]
struct GrandMA2PresetGroup {
    s: bool,
    fea: Vec<GrandMA2Feature>,
}

#[derive(Deserialize, Debug, Default)]
#[serde(default)]
struct GrandMA2Feature {
    att: Vec<GrandMA2Attribute>,
}

#[derive(Deserialize, Debug, Default)]
#[serde(default)]
struct GrandMA2Attribute {
    np: String,
    t: String,
    c: String,
}

#[derive(Deserialize, Debug, Default)]
#[serde(default, rename_all = "camelCase")]
struct GrandMA2PlaybacksResponse {
    item_groups: Vec<GrandMA2ItemGroup>,
}

#[derive(Deserialize, Debug, Default)]
#[serde(default)]
struct GrandMA2ItemGroup {
    items: Vec<Vec<GrandMA2ExecItem>>,
}

#[derive(Deserialize, Debug, Default)]
#[serde(default, rename_all = "camelCase")]
struct GrandMA2ExecItem {
    i_exec: u64,
    tt: GrandMA2Title,
    is_run: u64,
    executor_blocks: Vec<GrandMA2ExecBlock>,
    cues: GrandMA2Cues,
}

#[derive(Deserialize, Debug, Default)]
#[serde(default)]
struct GrandMA2Title {
    t: String,
}

#[derive(Deserialize, Debug, Default)]
#[serde(default)]
struct GrandMA2ExecBlock {
    button1: GrandMA2Title,
    fader: GrandMA2FaderInfo,
}

#[derive(Deserialize, Debug, Default)]
#[serde(default)]
struct GrandMA2FaderInfo {
    tt: String,
}

#[derive(Deserialize, Debug, Default)]
#[serde(default)]
struct GrandMA2Cues {
    items: Vec<GrandMA2CueItem>,
}

#[derive(Deserialize, Debug, Default)]
#[serde(default)]
struct GrandMA2CueItem {
    t: String,
    pgs: GrandMA2Pgs,
}

#[derive(Deserialize, Debug, Default)]
#[serde(default)]
struct GrandMA2Pgs {
    v: f64,
}

#[derive(Deserialize, Debug, Default)]
#[serde(default, rename_all = "camelCase")]
struct GrandMA2SystemResponse {
    response_type: String,
    status: String,
    force_login: bool,
    result: bool,
    session: u64,
}

#[derive(Deserialize, Debug)]
#[serde(tag = "responseType", rename_all = "camelCase")]
enum GrandMA2ServerMessage {
    PresetTypes(GrandMA2PresetTypesResponse),
    Playbacks(GrandMA2PlaybacksResponse),
    #[serde(untagged)]
    System(GrandMA2SystemResponse),
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct EncoderUpdate {
    pub value: String,
    pub active: bool,
    pub programmer: bool,
    pub group_active: bool,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub enum FaderFunction {
    #[default]
    Empty,
    Master,
    SpeedMaster,
    Temp,
    Other(String),
}

impl FaderFunction {
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "empty" | "" => FaderFunction::Empty,
            "master" => FaderFunction::Master,
            "speedmaster" => FaderFunction::SpeedMaster,
            "temp" | "tmp" => FaderFunction::Temp,
            _ => FaderFunction::Other(s.to_string()),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
pub enum ButtonFunction {
    #[default]
    Empty,
    Toggle,
    Flash,
    Temp,
    Learn, // SpeedMaster button
    Other(String),
}

impl ButtonFunction {
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "empty" | "" => ButtonFunction::Empty,
            "toggle" => ButtonFunction::Toggle,
            "flash" => ButtonFunction::Flash,
            "temp" | "tmp" => ButtonFunction::Temp,
            "learn" => ButtonFunction::Learn,
            _ => ButtonFunction::Other(s.to_string()),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct FaderExec {
    pub name: String,
    pub cue: String,
    pub active: bool,
    pub fade: Option<f32>,
    pub function: FaderFunction,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct ButtonExec {
    pub name: String,
    pub active: bool,
    pub function: ButtonFunction,
    pub bpm: Option<f32>,
    pub cue_name: Option<String>,
    pub fade: Option<f32>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ExecUpdate {
    Fader(usize, FaderExec),
    Button(usize, ButtonExec),
}

pub enum GrandMA2Event {
    PresetTypesUpdate(Vec<EncoderUpdate>),
    PlaybacksUpdate(Vec<ExecUpdate>),
}

pub struct GrandMA2Client;

impl GrandMA2Client {
    /// Launch the background auto-reconnecting grandMA2 WebRemote client.
    pub fn start(grandma2_tx: mpsc::Sender<GrandMA2Event>) -> mpsc::Sender<GrandMA2Command> {
        let args = CliArgs::parse();
        let config = GrandMA2Config::parse(&args.target);
        println!("[grandMA2] URL: {}", config.url());
        println!("[grandMA2] User: '{}' | Pass: '****'", config.username);

        let (commands_tx, mut commands_rx) = mpsc::channel::<GrandMA2Command>(100);

        tokio::spawn(async move {
            let pass_hash = format!("{:x}", md5::compute(config.password.as_bytes()));
            let url = config.url();

            loop {
                match connect_async(&url).await {
                    Ok((ws_stream, _)) => {
                        println!("[grandMA2] Connected to grandMA2 onPC at {}", url);

                        let (mut write, mut read) = ws_stream.split();

                        let mut session_id: u64 = 0;

                        // 1. Wait for server readiness message
                        while let Some(Ok(Message::Text(txt))) = read.next().await {
                            if let Ok(GrandMA2ServerMessage::System(resp)) =
                                serde_json::from_str::<GrandMA2ServerMessage>(&txt)
                            {
                                if resp.status == "server ready" {
                                    break;
                                }
                            }
                        }

                        // 2. Send initial session request {"session": 0} to prompt grandMA2 for a session ID
                        let init_payload = serde_json::json!({ "session": 0 });
                        let init_json = init_payload.to_string();

                        if let Err(e) = write.send(Message::Text(init_json)).await {
                            println!("[grandMA2] Session init failed: {}. Retrying...", e);
                            sleep(Duration::from_secs(2)).await;
                            continue;
                        }

                        if let Some(Ok(Message::Text(txt))) = read.next().await {
                            if let Ok(GrandMA2ServerMessage::System(resp)) =
                                serde_json::from_str::<GrandMA2ServerMessage>(&txt)
                            {
                                if resp.session > 0 {
                                    session_id = resp.session;
                                    println!("[grandMA2] Server assigned session ID: {}", session_id);
                                }
                            }
                        }

                        // 4. Send actual Login Request WITH the assigned session ID
                        let login_msg = GrandMA2LoginPayload {
                            request_type: "login".to_string(),
                            username: config.username.clone(),
                            password: pass_hash.clone(),
                            session: session_id,
                        };

                        let login_str = serde_json::to_string(&login_msg).unwrap_or_default();
                        if let Err(e) = write.send(Message::Text(login_str)).await {
                            println!("[grandMA2] Login failed: {}. Retrying...", e);
                            sleep(Duration::from_secs(2)).await;
                            continue;
                        }

                        if let Some(Ok(Message::Text(txt))) = read.next().await {
                            if let Ok(GrandMA2ServerMessage::System(resp)) =
                                serde_json::from_str::<GrandMA2ServerMessage>(&txt)
                            {
                                if resp.result {
                                    println!("[grandMA2] Logged in as '{}' (session: {})", config.username, session_id);
                                } else {
                                    println!("[grandMA2] Login rejected. Check username/password");
                                }
                            }
                        }

                        // 2. Active Session Loop: Handle outgoing commands and incoming messages
                        let mut keepalive_interval = tokio::time::interval(Duration::from_secs(10));
                        let mut poll_interval = tokio::time::interval(Duration::from_millis(200));

                        let ping_payload_str = serde_json::to_string(&serde_json::json!({
                            "session": session_id
                        }))
                        .unwrap_or_default();

                        let preset_req_str = serde_json::to_string(&serde_json::json!({
                            "requestType": "presetTypes",
                            "type": "update",
                            "session": session_id,
                            "maxRequests": 1
                        }))
                        .unwrap_or_default();

                        let playbacks_req_str = serde_json::to_string(&serde_json::json!({
                            "requestType": "playbacks",
                            "startIndex": [0],
                            "itemsCount": [15],
                            "pageIndex": 0,
                            "itemsType": [2],
                            "view": 2,
                            "execButtonViewMode": 1,
                            "buttonsViewMode": 0,
                            "session": session_id,
                            "maxRequests": 1
                        }))
                        .unwrap_or_default();

                        loop {
                            tokio::select! {
                                // Send periodic application-level JSON heartbeat frame every 10s: {"session": session_id}
                                _ = keepalive_interval.tick() => {
                                    if write.send(Message::Text(ping_payload_str.clone())).await.is_err() {
                                        println!("[grandMA2] Keep-alive failed");
                                        break;
                                    }
                                }

                                // Handle outgoing commands from hardware with coalescing
                                _ = poll_interval.tick() => {
                                    let _ = write.send(Message::Text(preset_req_str.clone())).await;
                                    let _ = write.send(Message::Text(playbacks_req_str.clone())).await;
                                }

                                Some(cmd) = commands_rx.recv() => {
                                    let mut cmds_to_process = vec![cmd];

                                    // Drain and collapse redundant fader movements & encoder steps in queue
                                    while let Ok(next_cmd) = commands_rx.try_recv() {
                                        let last_idx = cmds_to_process.len() - 1;
                                        match (&cmds_to_process[last_idx], &next_cmd) {
                                            (GrandMA2Command::GrandMaster { .. }, GrandMA2Command::GrandMaster { .. }) => {
                                                cmds_to_process[last_idx] = next_cmd;
                                            }
                                            (GrandMA2Command::ExecFader { exec_num: a, .. }, GrandMA2Command::ExecFader { exec_num: b, .. }) if a == b => {
                                                cmds_to_process[last_idx] = next_cmd;
                                            }
                                            (
                                                GrandMA2Command::Encoder { attribute: a1, delta: d1, fine: f1 },
                                                GrandMA2Command::Encoder { attribute: a2, delta: d2, fine: f2 },
                                            ) if a1 == a2 && f1 == f2 => {
                                                cmds_to_process[last_idx] = GrandMA2Command::Encoder {
                                                    attribute: a1.clone(),
                                                    delta: d1 + d2,
                                                    fine: *f1,
                                                };
                                            }
                                            _ => {
                                                cmds_to_process.push(next_cmd);
                                            }
                                        }
                                    }

                                    for c in cmds_to_process {
                                        let json_msg = match c {
                                            GrandMA2Command::HardKey { code, pressed } => {
                                                let cmd_str = format!("LUA 'gma.canbus.hardkey({}, {}, false)'", code, pressed);
                                                let payload = GrandMA2CmdPayload {
                                                    request_type: "command".to_string(),
                                                    command: cmd_str,
                                                    session: session_id,
                                                };
                                                serde_json::to_string(&payload).unwrap()
                                            }
                                            GrandMA2Command::Encoder { attribute, delta, fine } => {
                                                let sign_str = if delta >= 0 { "+ " } else { "- " };
                                                let abs_delta = delta.abs();

                                                let val_str = if attribute.eq_ignore_ascii_case("Dim") {
                                                    let step = if fine { 1 } else { 5 };
                                                    let num = abs_delta * step;
                                                    format!("{}{}", sign_str, num)
                                                } else {
                                                    if fine {
                                                        let num = (abs_delta as f32) * 0.1;
                                                        format!("{}{:.1}", sign_str, num)
                                                    } else {
                                                        let num = abs_delta * 2;
                                                        format!("{}{}", sign_str, num)
                                                    }
                                                };

                                                let cmd_str = format!("Attribute \"{}\" At {} If Selection", attribute, val_str);
                                                let payload = GrandMA2CmdPayload {
                                                    request_type: "command".to_string(),
                                                    command: cmd_str,
                                                    session: session_id,
                                                };
                                                serde_json::to_string(&payload).unwrap()
                                            }
                                            GrandMA2Command::ExecFader { exec_num, level } => {
                                                let norm_val = level as f64 / 127.0;
                                                let payload = serde_json::json!({
                                                    "requestType": "playbacks_userInput",
                                                    "execIndex": (exec_num.saturating_sub(1)) as u32,
                                                    "pageIndex": 0,
                                                    "faderValue": norm_val,
                                                    "type": 1,
                                                    "session": session_id
                                                });
                                                payload.to_string()
                                            }
                                            GrandMA2Command::ExecButton { exec_num, pressed } => {
                                                let payload = serde_json::json!({
                                                    "requestType": "playbacks_userInput",
                                                    "execIndex": (exec_num.saturating_sub(1)) as u32,
                                                    "pageIndex": 0,
                                                    "buttonId": 0,
                                                    "pressed": pressed,
                                                    "released": !pressed,
                                                    "type": 0,
                                                    "session": session_id
                                                });
                                                payload.to_string()
                                            }
                                            GrandMA2Command::GrandMaster { level } => {
                                                let pct = level as f32 / 127.0 * 100.0;
                                                let cmd_str = format!("SpecialMaster 2.1 At {:.1}", pct);
                                                let payload = GrandMA2CmdPayload {
                                                    request_type: "command".to_string(),
                                                    command: cmd_str,
                                                    session: session_id,
                                                };
                                                serde_json::to_string(&payload).unwrap()
                                            }
                                            GrandMA2Command::Command { cmd } => {
                                                let payload = GrandMA2CmdPayload {
                                                    request_type: "command".to_string(),
                                                    command: cmd,
                                                    session: session_id,
                                                };
                                                serde_json::to_string(&payload).unwrap()
                                            }
                                        };

                                        // Sending command (logging disabled to avoid lag)
                                        if write.send(Message::Text(json_msg)).await.is_err() {
                                            println!("[grandMA2] Command transmit failed");
                                            break;
                                        }
                                    }
                                }

                                // Read incoming WebRemote messages
                                msg = read.next() => {
                                    match msg {
                                        Some(Ok(Message::Text(txt))) => {
                                            if let Ok(server_msg) = serde_json::from_str::<GrandMA2ServerMessage>(&txt) {
                                                match server_msg {
                                                    GrandMA2ServerMessage::System(resp) => {
                                                        if resp.force_login {
                                                            let login_msg = GrandMA2LoginPayload {
                                                                request_type: "login".to_string(),
                                                                username: config.username.clone(),
                                                                password: pass_hash.clone(),
                                                                session: session_id,
                                                            };
                                                            let login_str = serde_json::to_string(&login_msg).unwrap_or_default();
                                                            let _ = write.send(Message::Text(login_str)).await;
                                                        }

                                                        let new_session_id = resp.session;
                                                        if new_session_id > 0 && new_session_id != session_id {
                                                            println!("[grandMA2] Session ID updated by server {} -> {}", session_id, new_session_id);
                                                            session_id = new_session_id;
                                                        }

                                                        if !resp.result && resp.response_type == "login" {
                                                            println!("[grandMA2] grandMA2 invalidated session {}. Re-authenticating...", session_id);
                                                            break;
                                                        }
                                                    }
                                                    GrandMA2ServerMessage::PresetTypes(res) => {
                                                        let encoder_update = parse_preset_types(res);
                                                        let _ = grandma2_tx.send(encoder_update).await;
                                                    }
                                                    GrandMA2ServerMessage::Playbacks(res) => {
                                                        if let Some(executor_update) = parse_playbacks(res) {
                                                            let _ = grandma2_tx.send(executor_update).await;
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                        Some(Ok(Message::Close(_))) | None => {
                                            println!("[grandMA2] Server closed connection");
                                            break;
                                        }
                                        Some(Err(e)) => {
                                            println!("[grandMA2] Connection error: {}", e);
                                            break;
                                        }
                                        _ => {}
                                    }
                                }
                            }
                        }
                    }
                    Err(e) => {
                        println!("[grandMA2] Could not connect: {}. Retrying...", e);
                    }
                }

                sleep(Duration::from_secs(2)).await;
            }
        });

        commands_tx
    }
}

fn parse_preset_types(res: GrandMA2PresetTypesResponse) -> GrandMA2Event {
    let mut encoders = vec![EncoderUpdate::default(); 4];

    for pre in res.pre {
        for fea in pre.fea {
            for att in fea.att {
                if let Some(i) = match att.np.to_ascii_lowercase().as_str() {
                    "dim" => Some(0),
                    "pan" => Some(1),
                    "tilt" => Some(2),
                    "zoom" => Some(3),
                    _ => None,
                } {
                    encoders[i] = EncoderUpdate {
                        active: true,
                        group_active: pre.s,
                        value: att.t.replace("-", ""),
                        programmer: att.c.eq_ignore_ascii_case("#FF0000"),
                    };
                }
            }
        }
    }

    GrandMA2Event::PresetTypesUpdate(encoders)
}

fn parse_playbacks(res: GrandMA2PlaybacksResponse) -> Option<GrandMA2Event> {
    let groups = res.item_groups.get(0)?;
    let mut execs = Vec::new();

    for block in &groups.items {
        for item in block {
            let idx = item.i_exec;
            if idx > 0 && idx <= 8 {
                let name = item.tt.t.clone();
                let active = item.is_run == 1;

                let mut cue = String::new();
                let mut fade = None;
                let mut bpm = None;
                let mut button_type = String::new();
                let mut fader_type = String::new();

                if let Some(first_block) = item.executor_blocks.get(0) {
                    button_type = first_block.button1.t.clone();
                    fader_type = first_block.fader.tt.clone();
                }

                let active_cue = item.cues.items.get(1).or_else(|| item.cues.items.get(0));
                if let Some(first_cue) = active_cue {
                    cue = first_cue.t.clone();

                    if cue.ends_with(" BPM") {
                        if let Ok(val) = cue.trim_end_matches(" BPM").parse::<f32>() {
                            bpm = Some(val);
                        }
                    }

                    for cue_item in &item.cues.items {
                        if cue_item.pgs.v > 0.0 {
                            fade = Some(cue_item.pgs.v as f32);
                            break;
                        }
                    }
                }

                let mut function = ButtonFunction::from_str(&button_type);
                if fader_type.eq_ignore_ascii_case("SpeedMaster") {
                    function = ButtonFunction::Learn;
                }

                if idx < 4 {
                    execs.push(ExecUpdate::Fader(
                        idx as usize,
                        FaderExec {
                            name: name.clone(),
                            cue: cue.clone(),
                            active,
                            fade,
                            function: FaderFunction::from_str(&fader_type),
                        },
                    ));
                } else if idx >= 5 && idx <= 8 {
                    execs.push(ExecUpdate::Button(
                        idx as usize,
                        ButtonExec {
                            name,
                            active,
                            function,
                            bpm,
                            cue_name: if cue.is_empty() { None } else { Some(cue) },
                            fade,
                        },
                    ));
                }
            }
        }
    }

    Some(GrandMA2Event::PlaybacksUpdate(execs))
}

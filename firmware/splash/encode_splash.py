from PIL import Image

# Open the PNG
img = Image.open("splash.png").convert("RGB")
pixels = img.load()

# Create byte array of 1024 bytes
splash_data = [0] * 1024

for y in range(64):
    for x in range(128):
        # We assume white is "on" (1) and anything else is "off" (0)
        # Check if the pixel is bright
        r, g, b = pixels[x, y]
        if r > 128 or g > 128 or b > 128:
            page = y // 8
            bit_pos = y % 8
            byte_idx = (page * 128) + x
            bit_mask = 1 << bit_pos
            splash_data[byte_idx] |= bit_mask

# Format as Rust array
out = "pub const SPLASH_DATA: [u8; 1024] = [\n"
for i in range(0, 1024, 16):
    chunk = splash_data[i:i+16]
    out += "    " + ", ".join([f"0x{b:02X}" for b in chunk]) + ",\n"
out += "];\n"

# Read displays.rs
file_path = "../src/displays.rs"
with open(file_path, "r") as f:
    content = f.read()

# Replace the array
start_marker = "pub const SPLASH_DATA: [u8; 1024] = ["
end_marker = "];"
start_idx = content.find(start_marker)
end_idx = content.find(end_marker, start_idx) + len(end_marker)

new_content = content[:start_idx] + out.strip() + content[end_idx:]

# Write back
with open(file_path, "w") as f:
    f.write(new_content)

print("Successfully updated displays.rs with new SPLASH_DATA")

import subprocess
import xml.etree.ElementTree as ET
from xml.dom import minidom

MA_LOGO_XML = """
<g id="MA_Logo">
    <rect x="0" y="0" fill="#000000" width="40" height="40"/>
    <rect x="42" y="0" fill="#000000" width="40" height="40"/>
    <path fill="#FFFFFF" d="M27.7,34.9l0.1-22.8h-0.1l-6,22.8h-3.9l-5.8-22.8h-0.1l0.1,22.8h-4.6v-29.7h6.8l5.7,21.7h0.2 l5.6-21.7h7.1v29.7H27.7z"/>
    <path fill="#FFFFFF" d="M69.1,35.1l-2.4-7.9h-9.7l-2.5,7.9h-5.2l9.9-30.2h5.8l9.6,30.2H69.1z M62.0,10.1 L62.0,10.1c0,0-3.1,10.8-3.8,12.8h7.4C65.3,22.3,62.2,11.3,62.0,10.1z"/>
</g>
"""

def generate_keycap_svg(u1keys, u2key, output_filename):
    # --- Configuration ---
    pitch = 21.0          
    border = 1.6          
    key_top_size = 12.0   
    font_size_mm = 3.5    
    text_y_offset = -3.0  
    
    two_u_width = 30.0
    two_u_height = 12.0
    two_u_dist_from_right = 21.65
    
    # If a 2U key exists at the bottom right, there are 23 available 1U slots
    # If no 2U key exists, there are 25 available 1U slots
    max_u1_slots = 23 if u2key is not None else 25
    strings = (u1keys + [""] * max_u1_slots)[:max_u1_slots]
    
    start_x = border + (pitch / 2.0)
    start_y = border + (pitch / 2.0)
    
    total_width = (border * 2) + (pitch * 5)
    total_height = (border * 2) + (pitch * 5)
    
    svg = ET.Element('svg', {
        'xmlns': 'http://www.w3.org/2000/svg',
        'width': f'{total_width}mm',
        'height': f'{total_height}mm',
        'viewBox': f'0 0 {total_width} {total_height}'
    })
    
    # --- Background Layer ---
    # Added white background to ensure correct rendering in all viewers
    ET.SubElement(svg, 'rect', {
        'width': '100%',
        'height': '100%',
        'fill': '#FFFFFF'
    })
    
    # Layer 1: Alignment Guidelines (Red)
    layer_alignment = ET.SubElement(svg, 'g', {
        'id': 'Alignment_Squares',
        'stroke': '#FF0000',
        'stroke-width': '0.03',
        'fill': 'none'
    })
    
    # Layer 2: Raster Text (Black)
    layer_text = ET.SubElement(svg, 'g', {
        'id': 'Engraving_Text',
        'fill': '#000000',
        'stroke': 'none',
        'font-family': "'Roboto Condensed', serif",
        'font-weight': '500',
        'font-size': f'{font_size_mm}px',
        'font-stretch': 'extra-condensed',
        'text-anchor': 'middle',
        'dominant-baseline': 'central'
    })
    
    u1_index = 0
    skip_next = False
    
    for row in range(5):
        for col in range(5):
            if skip_next:
                skip_next = False
                continue
                
            # If we hit the bottom right and a 2U key was provided
            if row == 4 and col == 3 and u2key is not None:
                cx = total_width - two_u_dist_from_right
                cy = start_y + (row * pitch)
                rect_w, rect_h = two_u_width, two_u_height
                key_content = u2key
                skip_next = True
            else:
                cx = start_x + (col * pitch)
                cy = start_y + (row * pitch)
                rect_w, rect_h = key_top_size, key_top_size
                key_content = strings[u1_index]
                u1_index += 1
            
            rect_x = cx - (rect_w / 2.0)
            rect_y = cy - (rect_h / 2.0)
            
            ET.SubElement(layer_alignment, 'rect', {
                'x': str(rect_x),
                'y': str(rect_y),
                'width': str(rect_w),
                'height': str(rect_h)
            })

            if key_content == "{MA_LOGO}":
                scale_factor = 10.0 / 82
                
                # Transform: Move to key center, apply scale, then shift back by half the logo's native size (44.4x20)
                wrapper = ET.SubElement(layer_text, 'g', {
                    'transform': f'translate({cx}, {cy + text_y_offset}) scale({scale_factor}) translate(-41, -20)'
                })
                wrapper.append(ET.fromstring(MA_LOGO_XML))
            elif key_content:
                text_element = ET.SubElement(layer_text, 'text', {
                    'x': str(cx),
                    'y': str(cy + text_y_offset)
                })
                text_element.text = key_content
            
    xmlstr = minidom.parseString(ET.tostring(svg)).toprettyxml(indent="  ")
    with open(output_filename, "w", encoding="utf-8") as f:
        f.write(xmlstr)
    print(f"Generated: {output_filename}")

    # --- Convert Text to Paths via Inkscape CLI ---
    try:
        # Inkscape v1.0+ CLI syntax
        subprocess.run([
            "inkscape", 
            output_filename, 
            "--export-text-to-path", 
            "--export-plain-svg", 
            "-o", output_filename
        ], check=True, capture_output=True)
        print(f"Converted text to paths: {output_filename}\n")
        
    except FileNotFoundError:
        print("⚠️ Inkscape not found in system PATH. Text remains as <text> elements.\n")
    except subprocess.CalledProcessError as e:
        print(f"⚠️ Failed to convert {output_filename}: {e.stderr.decode()}\n")


batch_1_keys = ["Blind", "Freeze", "Preview", "Tools", "Setup", "Backup", "Assign", "Align", "Help", "B/O", "Fix", "Select", "Off", "Temp", "Top", "On", "<<<", "Learn", ">>>", "Go-", "Pause", "Go+", "View"]
generate_keycap_svg(batch_1_keys, "Please", "batch_1.svg")

batch_2_keys = ["Effect", "Goto", "Page", "Macro", "Preset", "Sequ", "Cue", "Exec", "Channel", "Fixture", "Group", "Time", "Esc", "Edit", "Oops", "Update", "Clear", "Store", "Delete", "Copy", "Move", "+", "."]
generate_keycap_svg(batch_2_keys, "Pause", "batch_2.svg")

batch_3_keys = ["Thru", "-", "At", "If", "0", "1", "2", "3", "4", "5", "6", "7", "8", "9", "Full", "Highlt", "Solo", "Up", "Down", "Prev", "Set", "Next", "{MA_LOGO}"]
generate_keycap_svg(batch_3_keys, "Go -", "batch_3.svg")

batch_4_keys = ["X1", "X2", "X3", "X4", "X5", "X6", "X7", "X8", "X9", "X10", "X11", "X12", "X13", "X14", "X15", "X16", "X17", "Exec1", "Exec2", "Exec3", "Exec4"]
generate_keycap_svg(batch_4_keys, "Go", "batch_4.svg")

batch_5_keys = ["Position", "Dimmer", "Color", "Beam", "Focus", "Control", "V1", "V2", "V3", "V4"]
generate_keycap_svg(batch_5_keys, "Please", "batch_5.svg")

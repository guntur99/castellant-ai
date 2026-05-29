import re
import os

html_path = '/Users/guntur/Dev/rust/castellant/templates/invitation/create.html'
js_path = '/Users/guntur/Dev/rust/castellant/static/js/create.js'

with open(html_path, 'r') as f:
    content = f.read()

# Find the large script block at the end
script_pattern = re.compile(r'<script>\s*(let selectedGalleryImages.*?)<\/script>', re.DOTALL)
match = script_pattern.search(content)

if match:
    script_content = match.group(1)
    
    # Extract TPL_MAP
    tpl_map_pattern = re.compile(r'(// Dynamic Template Map from Database\s*const TPL_MAP = \{\s*\{% for t in all_templates %\}\s*\'\{\{ t\.id \}\}\': \'\{\{ t\.title \}\}\',\s*\{% endfor %\}\s*\};\s*)')
    tpl_match = tpl_map_pattern.search(script_content)
    
    if tpl_match:
        tpl_map_code = tpl_match.group(1)
        # Remove TPL_MAP from script_content
        script_content = script_content.replace(tpl_map_code, '')
        
        # Write to create.js
        os.makedirs(os.path.dirname(js_path), exist_ok=True)
        with open(js_path, 'w') as f:
            f.write(script_content)
            
        # Replace in HTML
        new_html_block = f"""<script>
    {tpl_map_code}</script>
<script src="/static/js/create.js" defer></script>"""
        
        new_content = content.replace(match.group(0), new_html_block)
        
        with open(html_path, 'w') as f:
            f.write(new_content)
        
        print("Successfully extracted JS and updated HTML.")
    else:
        print("TPL_MAP not found in script!")
else:
    print("Script block not found!")

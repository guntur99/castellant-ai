import os

html_path = '/Users/guntur/Dev/rust/castellant/templates/invitation/create.html'
js_path = '/Users/guntur/Dev/rust/castellant/static/js/create.js'

with open(html_path, 'r') as f:
    lines = f.readlines()

start_idx = -1
end_idx = -1

for i, line in enumerate(lines):
    if line.strip() == '<script>':
        # Let's find the large one, it should be the last one or around line 2028
        if i > 2000:
            start_idx = i
            break

if start_idx != -1:
    for i in range(start_idx + 1, len(lines)):
        if line.strip() == '</script>' or '</script>' in lines[i]:
            end_idx = i
            break

if start_idx != -1 and end_idx != -1:
    script_lines = lines[start_idx+1:end_idx]
    script_content = "".join(script_lines)
    
    # We need to extract TPL_MAP part
    tpl_start = -1
    tpl_end = -1
    for i, line in enumerate(script_lines):
        if 'const TPL_MAP = {' in line:
            tpl_start = i
        if tpl_start != -1 and '};' in line:
            tpl_end = i
            break
            
    if tpl_start != -1 and tpl_end != -1:
        tpl_map_lines = script_lines[tpl_start-1:tpl_end+1] # Include the comment above it
        tpl_map_code = "".join(tpl_map_lines)
        
        # Remove it from script_content
        script_content = script_content.replace(tpl_map_code, '')
        
        os.makedirs(os.path.dirname(js_path), exist_ok=True)
        with open(js_path, 'w') as f:
            f.write(script_content)
            
        new_html = lines[:start_idx] + ["<script>\n", tpl_map_code, "</script>\n<script src=\"/static/js/create.js\" defer></script>\n"] + lines[end_idx+1:]
        
        with open(html_path, 'w') as f:
            f.writelines(new_html)
        print("Success")
    else:
        print("TPL MAP not found")
else:
    print(f"Indices not found {start_idx} {end_idx}")

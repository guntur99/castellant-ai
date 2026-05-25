import re
import subprocess
import os

html_path = "/Users/guntur/Dev/rust/castellant/templates/invitation/manage.html"
with open(html_path, "r") as f:
    html_content = f.read()

# Extract all <script> blocks along with their attributes
script_blocks = re.findall(r'<script([^>]*)>(.*?)</script>', html_content, re.DOTALL)

print(f"Found {len(script_blocks)} script blocks.")

for i, (attrs, script) in enumerate(script_blocks):
    if 'src=' in attrs or 'src =' in attrs:
        print(f"Block {i+1}: Skipping (external library: {attrs.strip()})")
        continue
    
    # Save script to a temporary file
    temp_js_path = f"/Users/guntur/Dev/rust/castellant/scratch/temp_script_{i+1}.js"
    
    # Replace Askama syntax like "{{ ... }}" or '{{ ... }}' or {{ ... }} with a dummy value to make it valid JS
    clean_script = re.sub(r'"\{\{[^}]*\}\}"', '"dummy_value"', script)
    clean_script = re.sub(r"\'\{\{[^}]*\}\}\'", "'dummy_value'", clean_script)
    clean_script = re.sub(r'\{\{[^}]*\}\}', '"dummy_value"', clean_script)
    # Also replace {% ... %} blocks (Askama control flow)
    clean_script = re.sub(r'\{%[^%]*%\}', '', clean_script)
    
    with open(temp_js_path, "w") as tf:
        tf.write(clean_script)
    
    # Run node -c on the temporary file
    result = subprocess.run(["node", "-c", temp_js_path], capture_output=True, text=True)
    if result.returncode == 0:
        print(f"Block {i+1}: Syntax OK")
        if os.path.exists(temp_js_path):
            os.remove(temp_js_path)
    else:
        print(f"Block {i+1}: Syntax ERROR!")
        print(result.stderr)

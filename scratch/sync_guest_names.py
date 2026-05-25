import os
import re

dir_path = "/Users/guntur/Dev/rust/castellant/templates/invitation/"
replacements = {
    r"Ade Guntur Fitridullah & Partner": "{{ invitation.recipient_name }}",
    r"Ade Guntur Fitridullah<br>& Partner": "{{ invitation.recipient_name }}",
    r"Guest & Partner": "{{ invitation.recipient_name }}",
    r"Tamu Undangan": "{{ invitation.recipient_name }}",
}

for filename in os.listdir(dir_path):
    if filename.endswith(".html"):
        file_path = os.path.join(dir_path, filename)
        with open(file_path, 'r', encoding='utf-8') as f:
            content = f.read()
        
        new_content = content
        for pattern, replacement in replacements.items():
            new_content = re.sub(pattern, replacement, new_content)
        
        if new_content != content:
            print(f"Updating {filename}")
            with open(file_path, 'w', encoding='utf-8') as f:
                f.write(new_content)

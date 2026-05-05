import os
import re

dir_path = "/Users/guntur/Dev/rust/castellant/templates/invitation/"
replacements = {
    # Replace hardcoded date strings in JS
    r"new Date\('2026-05-24T08:00:00'\)": "new Date('{{ invitation.event_date_iso }}')",
    # Case where double quotes are used
    r'new Date\("2026-05-24T08:00:00"\)': "new Date('{{ invitation.event_date_iso }}')",
    # Case with spaces
    r"new Date\( '2026-05-24T08:00:00' \)": "new Date('{{ invitation.event_date_iso }}')",
    
    # Replace hardcoded date strings in HTML text
    r"24 Mei 2026": "{{ invitation.event_date }}",
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

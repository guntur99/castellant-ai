import os

dir_path = "/Users/guntur/Dev/rust/castellant/templates/invitation/"

def final_syntax_fix(content, filename):
    # Fix the syntax to use the new helper methods
    content = content.replace("{{ rsvp.name[0] }}", "{{ rsvp.initial() }}")
    content = content.replace("{{ rsvp.message }}", "{{ rsvp.display_message() }}")
    return content

for filename in os.listdir(dir_path):
    if filename.endswith(".html") and filename not in ["manage.html", "create.html", "templates_list.html"]:
        file_path = os.path.join(dir_path, filename)
        with open(file_path, 'r', encoding='utf-8') as f:
            content = f.read()
        
        new_content = final_syntax_fix(content, filename)
        
        if new_content != content:
            print(f"Fixing syntax in {filename}")
            with open(file_path, 'w', encoding='utf-8') as f:
                f.write(new_content)

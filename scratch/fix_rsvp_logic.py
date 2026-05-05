import os
import re

dir_path = "/Users/guntur/Dev/rust/castellant/templates/invitation/"

def fix_rsvp_list(content, filename):
    # 1. Fix variables { rsvp.name } -> {{ rsvp.name }}
    content = content.replace("{ rsvp.name[0] }", "{{ rsvp.name[0] }}")
    content = content.replace("{ rsvp.name }", "{{ rsvp.name }}")
    content = content.replace("{ rsvp.attendance }", "{{ rsvp.attendance }}")
    content = content.replace("{ rsvp.guests }", "{{ rsvp.guests }}")
    content = content.replace("{ rsvp.message }", "{{ rsvp.message }}")

    # 2. Fix the conditional block wrapping
    # We look for the start of the list
    for list_id in ["wishesList", "reviewsList"]:
        pattern = f'id="{list_id}"(.*?)>(.*?)</section>'
        match = re.search(pattern, content, re.DOTALL)
        if match:
            # We found the list and everything until the next section
            # We want to find the closing </div> of the list itself.
            # Usually it's the </div> before the next big block.
            inner = match.group(2)
            
            # If we already have the broken if/else/endif, let's clean it up
            if "{% if invitation.is_preview %}" in inner:
                # Find the start of the first dummy card
                # Dummy cards usually have "Ayu Rahmawati"
                dummy_start = inner.find('<div class="')
                if dummy_start == -1: dummy_start = inner.find('<div ')
                
                # We'll just construct a clean version
                # First, extract actual dummy cards (the ones with Ayu Rahmawati etc)
                # They are usually after the broken {% endif %} </div>
                broken_endif = inner.find('{% endif %}')
                if broken_endif != -1:
                    actual_dummy_content = inner[broken_endif + 11:]
                    # Remove the extra </div> that was wrongly inserted
                    actual_dummy_content = re.sub(r'^\s*</div>', '', actual_dummy_content)
                    
                    new_inner = f"""
                {{% if invitation.is_preview %}}
                {actual_dummy_content}
                {{% else %}}
                {{% for rsvp in invitation.rsvps %}}
                <div class="wish-card visible" style="margin-bottom: 10px; padding: 15px; background: rgba(255,255,255,0.05); border-radius: 10px; border: 1px solid rgba(255,255,255,0.1);">
                    <div style="display: flex; gap: 10px; align-items: center; margin-bottom: 8px;">
                        <div style="width: 35px; height: 35px; border-radius: 50%; background: #E50914; display: flex; align-items: center; justify-content: center; font-weight: bold; font-size: 14px;">{{{{ rsvp.name[0] }}}}</div>
                        <div>
                            <div style="font-weight: bold; font-size: 14px;">{{{{ rsvp.name }}}}</div>
                            <div style="font-size: 11px; opacity: 0.7;">{{{{ rsvp.attendance }}}} · {{{{ rsvp.guests }}}} tamu</div>
                        </div>
                    </div>
                    <div style="font-size: 13px; line-height: 1.5; font-style: italic;">"{{{{ rsvp.message }}}}"</div>
                </div>
                {{% endfor %}}
                {{% endif %}}
                """
                    # We need to find the correct end of the list </div>
                    # Let's use a simpler replacement: replace everything between list start and next section
                    # but keep the closing section tag.
                    full_match = match.group(0)
                    # The list container itself needs to be closed.
                    # Usually it's <div id="list"> ... </div>
                    # We'll replace the inner content of the div.
                    
                    # Search for the first </div> after the dummy content
                    # This is risky but likely to work if dummy cards are the only things there.
                    
                    # Actually, I'll just use a more surgical replace on the whole file
                    content = content.replace(inner, new_inner + "\n            ")

    return content

for filename in os.listdir(dir_path):
    if filename.endswith(".html") and filename not in ["manage.html", "create.html", "templates_list.html"]:
        file_path = os.path.join(dir_path, filename)
        with open(file_path, 'r', encoding='utf-8') as f:
            content = f.read()
        
        new_content = fix_rsvp_list(content, filename)
        
        if new_content != content:
            print(f"Fixing {filename}")
            with open(file_path, 'w', encoding='utf-8') as f:
                f.write(new_content)

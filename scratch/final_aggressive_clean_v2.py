import os
import re

dir_path = "/Users/guntur/Dev/rust/castellant/templates/invitation/"

def final_aggressive_clean(content, filename):
    for list_id in ["wishesList", "reviewsList"]:
        if f'id="{list_id}"' in content:
            # Pattern:
            # Group 1: The opening div tag
            #   Group 2: The ID (nested in Group 1)
            # Group 3: The closing marker (section or footer)
            pattern = f'(<div[^>]*id="({list_id})"[^>]*>).*?(</section>|<footer)'
            
            dummy_content = """
                {% if invitation.is_preview %}
                    <div class="wish-card visible" style="margin-bottom: 10px; padding: 15px; background: rgba(255,255,255,0.05); border-radius: 10px; border: 1px solid rgba(255,255,255,0.1); opacity: 0.7;">
                        <div style="display: flex; gap: 10px; align-items: center; margin-bottom: 8px;">
                            <div style="width: 35px; height: 35px; border-radius: 50%; background: #666; display: flex; align-items: center; justify-content: center; font-weight: bold; font-size: 14px;">A</div>
                            <div>
                                <div style="font-weight: bold; font-size: 14px;">Ayu Rahmawati (Contoh)</div>
                                <div style="font-size: 11px; opacity: 0.7;">Hadir · 2 tamu</div>
                            </div>
                        </div>
                        <div style="font-size: 13px; line-height: 1.5; font-style: italic;">"Selamat menempuh hidup baru! Semoga bahagia selalu."</div>
                    </div>
                {% else %}
                {% for rsvp in invitation.rsvps %}
                <div class="wish-card visible" style="margin-bottom: 15px; padding: 15px; background: rgba(255,255,255,0.05); border-radius: 12px; border: 1px solid rgba(255,255,255,0.1); box-shadow: 0 4px 15px rgba(0,0,0,0.1);">
                    <div style="display: flex; gap: 12px; align-items: center; margin-bottom: 10px;">
                        <div style="width: 40px; height: 40px; border-radius: 50%; background: linear-gradient(45deg, #E50914, #b00710); display: flex; align-items: center; justify-content: center; font-weight: bold; font-size: 16px; color: white; box-shadow: 0 2px 8px rgba(229, 9, 20, 0.4);">{{ rsvp.initial() }}</div>
                        <div>
                            <div style="font-weight: bold; font-size: 15px; color: #fff;">{{ rsvp.name }}</div>
                            <div style="font-size: 12px; color: #00AA13; font-weight: 500;">✓ {{ rsvp.attendance }} · {{ rsvp.guests }} tamu</div>
                        </div>
                    </div>
                    <div style="font-size: 14px; line-height: 1.6; font-style: italic; color: rgba(255,255,255,0.9); padding: 10px; background: rgba(255,255,255,0.03); border-radius: 8px;">"{{ rsvp.display_message() }}"</div>
                </div>
                {% endfor %}
                {% endif %}
                </div>
                """
            
            # Using a lambda to avoid backslash issues in replacement string
            new_content = re.sub(pattern, lambda m: m.group(1) + dummy_content + m.group(3), content, flags=re.DOTALL)
            if new_content != content:
                content = new_content

    return content

for filename in os.listdir(dir_path):
    if filename.endswith(".html") and filename not in ["manage.html", "create.html", "templates_list.html"]:
        file_path = os.path.join(dir_path, filename)
        with open(file_path, 'r', encoding='utf-8') as f:
            content = f.read()
        
        new_content = final_aggressive_clean(content, filename)
        
        if new_content != content:
            print(f"Aggressively cleaning {filename}")
            with open(file_path, 'w', encoding='utf-8') as f:
                f.write(new_content)

import os
import re

dir_path = "/Users/guntur/Dev/rust/castellant/templates/invitation/"

def clean_list_container(content, filename):
    # List of container IDs to fix
    for list_id in ["wishesList", "reviewsList"]:
        if f'id="{list_id}"' in content:
            # We want to find <div id="list_id" ...> ... </div>
            # but <div> can be nested. However, in these templates,
            # the wishesList is usually the last thing in its section.
            
            # Better: find the start tag
            start_tag_match = re.search(f'(<div[^>]*id="{list_id}"[^>]*>)', content)
            if start_tag_match:
                start_pos = start_tag_match.end()
                
                # Now we need to find the matching closing </div>.
                # Since these templates are somewhat consistent, we can look for
                # the </div> before the next </section> or <footer>.
                end_pos = content.find('</section>', start_pos)
                if end_pos == -1: end_pos = content.find('</div>\n            </section>', start_pos)
                if end_pos == -1: end_pos = content.find('<footer', start_pos)
                
                if end_pos != -1:
                    # Find the last </div> before the end_pos
                    actual_end = content.rfind('</div>', start_pos, end_pos + 6)
                    if actual_end != -1:
                        # We found the range!
                        
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
"""
                        content = content[:start_pos] + dummy_content + content[actual_end:]

    return content

for filename in os.listdir(dir_path):
    if filename.endswith(".html") and filename not in ["manage.html", "create.html", "templates_list.html"]:
        file_path = os.path.join(dir_path, filename)
        with open(file_path, 'r', encoding='utf-8') as f:
            content = f.read()
        
        new_content = clean_list_container(content, filename)
        
        if new_content != content:
            print(f"Cleaning {filename}")
            with open(file_path, 'w', encoding='utf-8') as f:
                f.write(new_content)

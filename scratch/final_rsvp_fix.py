import os
import re

dir_path = "/Users/guntur/Dev/rust/castellant/templates/invitation/"

def final_fix(content, filename):
    # 1. Clean up broken blocks
    # Remove any stray </div> and empty if/else blocks we created
    content = re.sub(r'{% if invitation.is_preview %}\s*(<!-- RSVPs will be loaded here -->)?\s*</div>', '', content)
    
    # 2. Re-identify the list and dummy cards
    for list_id in ["wishesList", "reviewsList"]:
        if f'id="{list_id}"' in content:
            # Find the div and its inner content
            match = re.search(f'id="{list_id}"(.*?)>(.*?)</section>', content, re.DOTALL)
            if match:
                inner = match.group(2)
                
                # Extract dummy cards (any div with wish-card or review-card)
                # but only if they contain the hardcoded names
                dummy_cards = []
                # Simple split by card class
                cards = re.split(r'(<div class="(wish-card|review-card).*?>)', inner)
                # cards[0] is everything before first card
                # cards[1] is the tag
                # cards[2] is the class name
                # cards[3] is the content
                
                # This is too complex. Let's just use a fixed dummy content for all
                new_dummy = """
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
                """
                
                new_inner = f"""
                {{% if invitation.is_preview %}}
                {new_dummy}
                {{% else %}}
                {{% for rsvp in invitation.rsvps %}}
                <div class="wish-card visible" style="margin-bottom: 15px; padding: 15px; background: rgba(255,255,255,0.05); border-radius: 12px; border: 1px solid rgba(255,255,255,0.1); box-shadow: 0 4px 15px rgba(0,0,0,0.1);">
                    <div style="display: flex; gap: 12px; align-items: center; margin-bottom: 10px;">
                        <div style="width: 40px; height: 40px; border-radius: 50%; background: linear-gradient(45deg, #E50914, #b00710); display: flex; align-items: center; justify-content: center; font-weight: bold; font-size: 16px; color: white; box-shadow: 0 2px 8px rgba(229, 9, 20, 0.4);">{{{{ rsvp.name[0] }}}}</div>
                        <div>
                            <div style="font-weight: bold; font-size: 15px; color: #fff;">{{{{ rsvp.name }}}}</div>
                            <div style="font-size: 12px; color: #00AA13; font-weight: 500;">✓ {{{{ rsvp.attendance }}}} · {{{{ rsvp.guests }}}} tamu</div>
                        </div>
                    </div>
                    <div style="font-size: 14px; line-height: 1.6; font-style: italic; color: rgba(255,255,255,0.9); padding: 10px; background: rgba(255,255,255,0.03); border-radius: 8px;">"{{{{ rsvp.message }}}}"</div>
                </div>
                {{% endfor %}}
                {{% endif %}}
                """
                
                # Find the container again to replace inner content
                # We'll search for the tag and its closing </div>
                # This is safer:
                content = re.sub(f'(id="{list_id}"[^>]*>).*?(</div)', r'\1' + new_inner + r'\2', content, flags=re.DOTALL)

    return content

for filename in os.listdir(dir_path):
    if filename.endswith(".html") and filename not in ["manage.html", "create.html", "templates_list.html"]:
        file_path = os.path.join(dir_path, filename)
        with open(file_path, 'r', encoding='utf-8') as f:
            content = f.read()
        
        new_content = final_fix(content, filename)
        
        if new_content != content:
            print(f"Finalizing {filename}")
            with open(file_path, 'w', encoding='utf-8') as f:
                f.write(new_content)

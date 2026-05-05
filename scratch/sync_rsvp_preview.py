import os
import re

dir_path = "/Users/guntur/Dev/rust/castellant/templates/invitation/"

def update_rsvp_with_preview(content, filename):
    # 1. Update Stats
    stat_nums = re.findall(r'<div class="rsvp-stat-num">(\d+)</div>', content)
    if len(stat_nums) >= 3:
        content = re.sub(r'<div class="rsvp-stat-num">\d+</div>', r'<div class="rsvp-stat-num">{{ invitation.total_rsvps() }}</div>', content, 1)
        content = re.sub(r'<div class="rsvp-stat-num">\d+</div>', r'<div class="rsvp-stat-num">{{ invitation.total_hadir() }}</div>', content, 1)
        content = re.sub(r'<div class="rsvp-stat-num">\d+</div>', r'<div class="rsvp-stat-num">{{ invitation.total_guest_count() }}</div>', content, 1)

    # 2. Add fetch to submitRSVP (with is_preview check to prevent saving dummy data)
    if "function submitRSVP()" in content and "fetch('/api/rsvp'" not in content:
        fetch_call = """
            if (!'{{ invitation.is_preview }}' === 'true') {
                fetch('/api/rsvp', {
                    method: 'POST',
                    headers: { 'Content-Type': 'application/x-www-form-urlencoded' },
                    body: `invitation_slug={{ invitation.slug }}&name=${encodeURIComponent(name)}&attendance=${encodeURIComponent(status)}&guests=${typeof guestCount !== 'undefined' ? guestCount : 1}&message=${encodeURIComponent(msg)}`
                });
            }
"""
        insertion_point = re.search(r'const msg = .*?;', content)
        if insertion_point:
            pos = insertion_point.end()
            content = content[:pos] + fetch_call + content[pos:]

    # 3. Handle Wishes/Reviews List
    for list_id in ["wishesList", "reviewsList"]:
        list_match = re.search(f'id="{list_id}"(.*?)>(.*?)</div', content, re.DOTALL)
        if list_match:
            inner_content = list_match.group(2)
            # If there's already a loop or conditional, skip
            if "{% for" in inner_content or "{% if" in inner_content:
                continue
            
            # We'll use a generic RSVP card style for actual RSVPs
            # But the user said "buat kosong kalo bukan preview"
            # So we'll wrap current content in if is_preview
            new_inner = f"""
                {{% if invitation.is_preview %}}
                {inner_content}
                {{% else %}}
                {{% for rsvp in invitation.rsvps %}}
                <div class="wish-card visible" style="margin-bottom: 10px; padding: 15px; background: rgba(255,255,255,0.05); border-radius: 10px; border: 1px solid rgba(255,255,255,0.1);">
                    <div style="display: flex; gap: 10px; align-items: center; margin-bottom: 8px;">
                        <div style="width: 35px; height: 35px; border-radius: 50%; background: #E50914; display: flex; align-items: center; justify-content: center; font-weight: bold; font-size: 14px;">{{ rsvp.name[0] }}</div>
                        <div>
                            <div style="font-weight: bold; font-size: 14px;">{{ rsvp.name }}</div>
                            <div style="font-size: 11px; opacity: 0.7;">{{ rsvp.attendance }} · {{ rsvp.guests }} tamu</div>
                        </div>
                    </div>
                    <div style="font-size: 13px; line-height: 1.5; font-style: italic;">"{{ rsvp.message }}"</div>
                </div>
                {{% endfor %}}
                {{% endif %}}
"""
            content = content.replace(inner_content, new_inner)

    return content

for filename in os.listdir(dir_path):
    if filename.endswith(".html") and filename not in ["manage.html", "create.html", "templates_list.html"]:
        file_path = os.path.join(dir_path, filename)
        with open(file_path, 'r', encoding='utf-8') as f:
            content = f.read()
        
        new_content = update_rsvp_with_preview(content, filename)
        
        if new_content != content:
            print(f"Updating {filename}")
            with open(file_path, 'w', encoding='utf-8') as f:
                f.write(new_content)

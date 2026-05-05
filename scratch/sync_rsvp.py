import os
import re

dir_path = "/Users/guntur/Dev/rust/castellant/templates/invitation/"

# Patterns to find and replace
patterns = [
    # 1. Update stats (Respons, Hadir, Total Tamu)
    # This is tricky because the order matters.
    # Usually: 
    # 1st stat-num -> total_rsvps()
    # 2nd stat-num -> total_hadir()
    # 3rd stat-num -> total_guest_count()
]

def update_rsvp(content, filename):
    # Update Stats
    stat_nums = re.findall(r'<div class="rsvp-stat-num">(\d+)</div>', content)
    if len(stat_nums) >= 3:
        # Replace 1st occurrence
        content = re.sub(r'<div class="rsvp-stat-num">\d+</div>', r'<div class="rsvp-stat-num">{{ invitation.total_rsvps() }}</div>', content, 1)
        # Replace 2nd occurrence
        content = re.sub(r'<div class="rsvp-stat-num">\d+</div>', r'<div class="rsvp-stat-num">{{ invitation.total_hadir() }}</div>', content, 1)
        # Replace 3rd occurrence
        content = re.sub(r'<div class="rsvp-stat-num">\d+</div>', r'<div class="rsvp-stat-num">{{ invitation.total_guest_count() }}</div>', content, 1)
    elif len(stat_nums) == 2:
        content = re.sub(r'<div class="rsvp-stat-num">\d+</div>', r'<div class="rsvp-stat-num">{{ invitation.total_rsvps() }}</div>', content, 1)
        content = re.sub(r'<div class="rsvp-stat-num">\d+</div>', r'<div class="rsvp-stat-num">{{ invitation.total_hadir() }}</div>', content, 1)

    # 2. Add fetch to submitRSVP
    # We look for the closing brace of submitRSVP function or just before showToast
    if "function submitRSVP()" in content:
        fetch_call = """
            fetch('/api/rsvp', {
                method: 'POST',
                headers: { 'Content-Type': 'application/x-www-form-urlencoded' },
                body: `invitation_slug={{ invitation.slug }}&name=${encodeURIComponent(name)}&attendance=${encodeURIComponent(status)}&guests=${typeof guestCount !== 'undefined' ? guestCount : 1}&message=${encodeURIComponent(msg)}`
            });
"""
        # Insert before the last closing brace of the function or before the last line
        # A simpler way: insert after the name/status/msg definitions
        insertion_point = re.search(r'const msg = .*?;', content)
        if insertion_point:
            pos = insertion_point.end()
            content = content[:pos] + fetch_call + content[pos:]

    # 3. Empty the list and add loop
    # We look for <div ... id="wishesList">...</div> or reviewsList
    for list_id in ["wishesList", "reviewsList"]:
        list_match = re.search(f'id="{list_id}"(.*?)>(.*?)</div', content, re.DOTALL)
        if list_match:
            # We want to replace the inner content with a loop
            # But we don't know the card structure. 
            # So we'll just empty it as requested "bersihkan saja"
            inner_content = list_match.group(2)
            content = content.replace(inner_content, "\n                <!-- RSVPs will be loaded here -->\n            ")

    return content

for filename in os.listdir(dir_path):
    if filename.endswith(".html") and filename not in ["manage.html", "create.html", "templates_list.html"]:
        file_path = os.path.join(dir_path, filename)
        with open(file_path, 'r', encoding='utf-8') as f:
            content = f.read()
        
        new_content = update_rsvp(content, filename)
        
        if new_content != content:
            print(f"Updating {filename}")
            with open(file_path, 'w', encoding='utf-8') as f:
                f.write(new_content)

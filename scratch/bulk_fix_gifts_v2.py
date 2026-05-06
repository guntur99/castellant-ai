import os
import re

templates_dir = "/Users/guntur/Dev/rust/castellant/templates/invitation"

def fix_file(path):
    with open(path, "r") as f:
        content = f.read()
    
    if "8210" not in content and "1192" not in content:
        return False

    # Expanded patterns to catch more variants
    containers = [
        r'<div class="gift-cards">([\s\S]+?)</div>\s*</section>',
        r'<div class="gift-grid">([\s\S]+?)</div>\s*</section>',
        r'<div class="gift-list">([\s\S]+?)</div>\s*</section>',
        r'<div class="escrow-accounts">([\s\S]+?)</div>\s*</div>',
        r'<div class="gift-cards">([\s\S]+?)</div>\s*</div>',
        r'<div class="gift-grid">([\s\S]+?)</div>\s*</div>',
        r'<div class="gift-list">([\s\S]+?)</div>\s*</div>',
    ]
    
    found = False
    for pattern in containers:
        match = re.search(pattern, content)
        if match:
            inner_html = match.group(1)
            if "8210" in inner_html or "1192" in inner_html:
                # Find the first card (child div)
                # We look for a div that starts right after white space
                card_matches = re.finditer(r'<div class="[\w-]+">[\s\S]+?</div>\s+(?=<div|$)', inner_html)
                # Actually, most templates have very similar card structure
                # Let's just find the first <div class="..."> ... </div> block
                card_match = re.search(r'(<div class="[\w-]+">[\s\S]+?</div>)', inner_html)
                if card_match:
                    card_template = card_match.group(1)
                    
                    # Fix the template
                    t = card_template
                    t = re.sub(r"BCA|BNI", "{{ account.bank_name }}", t)
                    # Use a more generic regex for name to avoid missing any
                    # Usually it's between two tags or in a div
                    t = re.sub(r"Guntur Putra|Nazma Putri|Anita Pangestuti|Zardarian Ahadika N|Ade Guntur", "{{ account.account_holder }}", t)
                    t = re.sub(r"8210\s*3705\s*61|1192\s*9784\s*03", "{{ account.account_number }}", t)
                    # Fix JS functions
                    t = re.sub(r"(copyText|cp|copyNum)\(['\"]8210370561['\"]", r"\1('{{ account.account_number }}'", t)
                    t = re.sub(r"(copyText|cp|copyNum)\(['\"]1192978403['\"]", r"\1('{{ account.account_number }}'", t)
                    # Catch the raw numbers if they still exist
                    t = re.sub(r"8210370561|1192978403", "{{ account.account_number }}", t)
                    
                    loop = f"\n                    {{% for account in invitation.gift_accounts %}}\n                    {t}\n                    {{% endfor %}}\n                "
                    
                    # Replace the entire inner_html with the loop
                    new_content = content.replace(inner_html, loop)
                    with open(path, "w") as f:
                        f.write(new_content)
                    print(f"Fixed {os.path.basename(path)}")
                    found = True
                    break
    
    if not found:
        print(f"FAILED to find container in {os.path.basename(path)}")
    return found

for filename in os.listdir(templates_dir):
    if filename.endswith(".html"):
        if filename in ["cinemarry.html", "loveanthem.html", "we-manhua.html"]:
            continue
        fix_file(os.path.join(templates_dir, filename))

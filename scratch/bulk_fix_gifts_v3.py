import os
import re

templates_dir = "/Users/guntur/Dev/rust/castellant/templates/invitation"

def fix_file(path):
    try:
        with open(path, "r") as f:
            content = f.read()
    except:
        return False
    
    if "8210" not in content and "1192" not in content:
        return False

    # Find the first card to locate the container
    card_pattern = r'(<div[^>]*class="[\w-]+"[^>]*>[\s\S]*?(8210|1192)[\s\S]*?</div>)'
    card_match = re.search(card_pattern, content)
    if not card_match:
        print(f"No card found in {os.path.basename(path)}")
        return False
    
    card_start = card_match.start()
    
    # Find the parent div start
    parent_div_match = None
    # Look for a div that seems to be a container
    for m in re.finditer(r'<div[^>]*class="[^"]*(gift|escrow|cards|grid|list|container)[^"]*"[^>]*>', content[:card_start]):
        parent_div_match = m
    
    if not parent_div_match:
        # Take the most recent div
        for m in re.finditer(r'<div[^>]*class="[\w-]+"[^>]*>', content[:card_start]):
            parent_div_match = m
            
    if not parent_div_match:
        print(f"No parent div found in {os.path.basename(path)}")
        return False
    
    parent_end_tag_pos = parent_div_match.end()
    
    # Find the end of the container (must include all sample accounts)
    # Look for the last sample account
    last_pos = 0
    for m in re.finditer(r"8210|1192", content):
        last_pos = m.end()
    
    # Find the first closing div after the last account
    container_end_match = re.search(r'</div>', content[last_pos:])
    if not container_end_match:
        print(f"No container end found in {os.path.basename(path)}")
        return False
    
    container_end_pos = last_pos + container_end_match.start()
    
    inner_html = content[parent_end_tag_pos:container_end_pos]
    
    # Extract the template from the first card
    t = card_match.group(1)
    
    # Sanitize the template
    t = re.sub(r"Bank\s*BCA|Bank\s*BNI|BCA|BNI", "{{ account.bank_name }}", t)
    t = re.sub(r"Guntur Putra|Nazma Putri|Anita Pangestuti|Zardarian Ahadika N|Ade Guntur|Nazma Putri", "{{ account.account_holder }}", t)
    t = re.sub(r"8210\s*3705\s*61|1192\s*9784\s*03", "{{ account.account_number }}", t)
    t = re.sub(r"(copyText|cp|copyNum)\(['\"]8210370561['\"]", r"\1('{{ account.account_number }}'", t)
    t = re.sub(r"(copyText|cp|copyNum)\(['\"]1192978403['\"]", r"\1('{{ account.account_number }}'", t)
    t = re.sub(r"8210370561|1192978403", "{{ account.account_number }}", t)
    
    # If the template still contains sample numbers in some other format, replace them
    t = re.sub(r"\d{4}\s*\d{4}\s*\d{2}", "{{ account.account_number }}", t)

    loop = f"\n                    {{% for account in invitation.gift_accounts %}}\n                    {t}\n                    {{% endfor %}}\n                "
    
    new_content = content[:parent_end_tag_pos] + loop + content[container_end_pos:]
    
    with open(path, "w") as f:
        f.write(new_content)
    print(f"SUCCESS: Fixed {os.path.basename(path)}")
    return True

# Running the script
for filename in sorted(os.listdir(templates_dir)):
    if filename.endswith(".html"):
        if filename in ["cinemarry.html", "loveanthem.html", "we-manhua.html"]:
            continue
        fix_file(os.path.join(templates_dir, filename))

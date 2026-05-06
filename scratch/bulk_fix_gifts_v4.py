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

    # Find all blocks that look like a gift card
    # A gift card is a div that contains one of the sample numbers
    # We look for <div ...> ... (8210|1192) ... </div>
    # Note: we need to handle nested divs, but usually these cards are simple.
    card_pattern = r'(<div[^>]*class="[\w-]+"[^>]*>[\s\S]*?(8210|1192)[\s\S]*?</div>)'
    
    cards = list(re.finditer(card_pattern, content))
    if not cards:
        print(f"No cards found in {os.path.basename(path)}")
        return False
    
    # Determine the range of the cards
    # If they are consecutive (with only whitespace between them), we replace the whole block
    start_pos = cards[0].start()
    end_pos = cards[-1].end()
    
    # Check if there's anything other than whitespace between cards
    # (Actually, let's just assume they are the cards we want to replace)
    
    # Extract template from the first card
    t = cards[0].group(1)
    
    # Sanitize the template
    # Bank name
    t = re.sub(r"Bank\s*BCA|Bank\s*BNI|BCA|BNI", "{{ account.bank_name }}", t)
    # Account holder
    t = re.sub(r"Guntur Putra|Nazma Putri|Anita Pangestuti|Zardarian Ahadika N|Ade Guntur|Guntur Fitridullah|Nazma", "{{ account.account_holder }}", t)
    # Account number (formatted)
    t = re.sub(r"8210\s*3705\s*61|1192\s*9784\s*03", "{{ account.account_number }}", t)
    # JS calls
    t = re.sub(r"(copyText|cp|copyNum)\(['\"]8210370561['\"]", r"\1('{{ account.account_number }}'", t)
    t = re.sub(r"(copyText|cp|copyNum)\(['\"]1192978403['\"]", r"\1('{{ account.account_number }}'", t)
    # Raw numbers
    t = re.sub(r"8210370561|1192978403", "{{ account.account_number }}", t)
    # Any other 10-digit number that looks like an account
    t = re.sub(r"\d{4}\s*\d{4}\s*\d{2}", "{{ account.account_number }}", t)

    loop = f"\n                    {{% for account in invitation.gift_accounts %}}\n                    {t}\n                    {{% endfor %}}\n                "
    
    new_content = content[:start_pos] + loop + content[end_pos:]
    
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

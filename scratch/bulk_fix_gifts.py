import os
import re
from bs4 import BeautifulSoup

templates_dir = "/Users/guntur/Dev/rust/castellant/templates/invitation"

def fix_file(path):
    with open(path, "r") as f:
        content = f.read()
    
    if "8210" not in content and "1192" not in content:
        return False

    # Use BeautifulSoup to parse and find the container
    soup = BeautifulSoup(content, 'html.parser')
    
    # Find all elements containing the sample number
    samples = soup.find_all(string=re.compile(r"8210|1192"))
    if not samples:
        return False
    
    # Find the common parent of at least two such elements
    # Usually the container of gift cards
    container = None
    for s in samples:
        p = s.parent
        while p:
            # Look for a parent that has children which look like cards
            # A common pattern is a div with 2 or more children
            children = [c for c in p.children if c.name == 'div']
            if len(children) >= 2:
                # Check if this container contains the sample text
                if "8210" in p.get_text() and "1192" in p.get_text():
                    container = p
                    break
            p = p.parent
        if container:
            break
            
    if not container:
        print(f"Could not find container in {os.path.basename(path)}")
        return False

    # Now we have the container. Let's take the first card as a template.
    cards = [c for c in container.children if c.name == 'div']
    if not cards:
        return False
    
    template_card = cards[0]
    
    # Stringify the template card
    card_str = str(template_card)
    
    # Replace sample data with template tags
    # Bank name (BCA/BNI)
    card_str = re.sub(r"BCA|BNI", "{{ account.bank_name }}", card_str)
    # Account holder (contains Guntur, Nazma, Anita, Zarda, etc.)
    card_str = re.sub(r"Guntur Putra|Nazma Putri|Anita Pangestuti|Zardarian Ahadika N|Ade Guntur", "{{ account.account_holder }}", card_str)
    # Account number (with spaces)
    card_str = re.sub(r"8210\s*3705\s*61|1192\s*9784\s*03", "{{ account.account_number }}", card_str)
    # Account number (without spaces, for JS calls)
    card_str = re.sub(r"['\"]8210370561['\"]|['\"]1192978403['\"]", "'{{ account.account_number }}'", card_str)
    # Also handle raw numbers if they appear in onclick
    card_str = re.sub(r"copyText\('8210370561'", "copyText('{{ account.account_number }}'", card_str)
    card_str = re.sub(r"cp\('8210370561'", "cp('{{ account.account_number }}'", card_str)
    card_str = re.sub(r"copyNum\('8210370561'", "copyNum('{{ account.account_number }}'", card_str)

    # Final loop block
    loop_block = "\n        {% for account in invitation.gift_accounts %}\n        " + card_str + "\n        {% endfor %}\n    "
    
    # Replace the container's inner HTML
    # This is tricky because BeautifulSoup's string representation might differ from original
    # So we'll do a string-based replacement of the whole container content if possible
    
    orig_container_str = str(container)
    # We want to keep the container tag but replace everything inside
    
    # Create a new BeautifulSoup object for the loop
    loop_soup = BeautifulSoup(loop_block, 'html.parser')
    container.clear()
    container.append(loop_soup)
    
    # Since BeautifulSoup might mess up the whole document's formatting/indentation,
    # and might escape template tags (though it shouldn't if they are just text),
    # let's be careful.
    
    # Actually, a better way to replace in the original file:
    # Find the start and end of the container in the original content.
    
    # But for now let's try the soup way and see if it works.
    # Note: BeautifulSoup will escape {{ }} if we are not careful.
    
    # Wait, I'll just use raw string replacement for the container inner part.
    # I'll find the container tag in the original file.
    
    return True

# Running the script
for filename in os.listdir(templates_dir):
    if filename.endswith(".html"):
        # Special skip for already fixed ones
        if filename in ["cinemarry.html", "loveanthem.html", "we-manhua.html"]:
            continue
        fix_file(os.path.join(templates_dir, filename))

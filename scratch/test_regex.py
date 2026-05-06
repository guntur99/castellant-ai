import re

content = """
    <div class="section-card" id="gift">
      <div class="escrow-header">
        <div class="escrow-icon">
          <svg viewBox="0 0 24 24"><path d="M20 4H4c-1.11 0-1.99.89-1.99 2L2 18c0 1.11.89 2 2 2h16c1.11 0 2-.89 2-2V6c0-1.11-.89-2-2-2zm0 14H4v-6h16v6zm0-10H4V6h16v2z"/></svg>
        </div>
        <div>
          <div class="escrow-title">Escrow Payment — Amplop Digital</div>
          <div class="escrow-sub">Secured · Verified Accounts · Instant Transfer</div>
        </div>
      </div>
      <div class="escrow-desc">Doa restu sudah sangat cukup, namun jika memberi merupakan tanda kasih, kami dengan senang hati menerimanya melalui rekening berikut.</div>
      <div class="escrow-accounts">
        <div class="escrow-account">
"""

card_start = content.find('<div class="escrow-account">')
print(f"Card start: {card_start}")

parent_div_match = None
for m in re.finditer(r'<div[^>]*class="[^"]*(gift|escrow|cards|grid|list|container)[^"]*"[^>]*>', content[:card_start]):
    print(f"Found match: {m.group(0)}")
    parent_div_match = m

if not parent_div_match:
    print("No parent div found")
else:
    print(f"Final parent: {parent_div_match.group(0)}")

import os
import re

templates_dir = "/Users/guntur/Dev/rust/castellant/templates/invitation"

for filename in os.listdir(templates_dir):
    if not filename.endswith(".html"):
        continue
    
    path = os.path.join(templates_dir, filename)
    with open(path, "r") as f:
        content = f.read()
    
    if "8210" in content and "1192" in content:
        print(f"Found sample accounts in {filename}")
        # Try to find the container
        # Pattern 1: <div class="gift-cards">...</div>
        # Pattern 2: <div class="gift-grid">...</div>
        # Pattern 3: <div class="gift-list">...</div>

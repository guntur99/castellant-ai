import os
import re

path = '/Users/guntur/Dev/rust/castellant/templates/invitation/manage.html'
with open(path, 'r') as f:
    content = f.read()

# Replace Music Library label
old_label = re.compile(r'<label\s+style="display: flex; justify-content: space-between; align-items: center; margin-bottom: 0\.8rem; font-weight: 600; color: #4A3728;">\s+<span style="font-size: 1rem;">Lagu Background \(Library\)</span>.*?</label>', re.DOTALL)
new_label = """                                     <div style="display: flex; justify-content: space-between; align-items: flex-end; margin-bottom: 1.2rem;">
                                         <h4 class="serif" style="color: #4A3728; font-size: 1.3rem; margin: 0;">Lagu Background (Library)</h4>
                                         <span style="font-size: 0.8rem; color: #D4A373; background: #FEF9E7; padding: 6px 14px; border-radius: 50px; border: 1px solid #FCF3CF; font-weight: 700; letter-spacing: 0.3px;">Pilih dari koleksi kami</span>
                                     </div>"""
content = old_label.sub(new_label, content)

# Replace Music Library select div
old_select = re.compile(r'<div style="position: relative;">\s+<select name="song_id".*?</select>\s+<div\s+style="position: absolute; right: 20px; top: 50%; transform: translateY\(-50%\); pointer-events: none; color: #D4A373;">\s+<i class="fas fa-chevron-down"></i>\s+</div>\s+</div>', re.DOTALL)
new_select = """                                     <div style="position: relative;">
                                         <select name="song_id"
                                             style="width: 100%; padding: 16px 22px; border-radius: 20px; border: 1.5px solid rgba(212, 163, 115, 0.15); outline: none; font-size: 1rem; background: white; appearance: none; transition: all 0.3s; cursor: pointer; color: #4A3728; box-shadow: 0 4px 15px rgba(0,0,0,0.03);">
                                             <option value="">Pilih Lagu...</option>
                                             {% for song in all_songs %}
                                             <option value="{{ song.id }}" {% if invitation.song_id|eq_uuid_opt(song.id)
                                                 %}selected{% endif %}>
                                                 {{ song.title }} — {{ song.artist }}
                                             </option>
                                             {% endfor %}
                                         </select>
                                         <div
                                             style="position: absolute; right: 22px; top: 50%; transform: translateY(-50%); pointer-events: none; color: #D4A373; font-size: 0.9rem;">
                                             <i class="fas fa-chevron-down"></i>
                                         </div>
                                     </div>"""
content = old_select.sub(new_select, content)

# Replace Playlist label
old_playlist_label = re.compile(r'<label\s+style="display: flex; justify-content: space-between; align-items: center; margin-bottom: 0\.8rem; font-weight: 600; color: #4A3728;">\s+<span style="font-size: 1rem;">Daftar Lagu \(Playlist\)</span>.*?</label>', re.DOTALL)
new_playlist_label = """                                         <div style="display: flex; justify-content: space-between; align-items: flex-end; margin-bottom: 1.2rem;">
                                             <h4 class="serif" style="color: #4A3728; font-size: 1.3rem; margin: 0;">Daftar Lagu (Playlist)</h4>
                                             <span style="font-size: 0.8rem; color: #94a3b8; font-weight: 600;">Limit Paket: {{ invitation.plan_name }}</span>
                                         </div>"""
content = old_playlist_label.sub(new_playlist_label, content)

with open(path, 'w') as f:
    f.write(content)

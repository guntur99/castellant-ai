import os

path = '/Users/guntur/Dev/rust/castellant/templates/invitation/manage.html'
with open(path, 'r') as f:
    content = f.read()

# Target block to replace (the old video section)
# We match from the old label to the end of the focal point slider div.
import re

pattern = r'<label\s+style="display: flex; justify-content: space-between; align-items: center; margin-bottom: 0\.8rem; font-weight: 600; color: #4A3728;">\s+<span style="font-size: 1rem;">Upload Video \(MP4\)</span>.*?</script>\s+</div>'
# This might be too complex for a single regex if there are nested divs.

# Let's try to replace the label first.
content = re.sub(r'<label\s+style="display: flex; justify-content: space-between; align-items: center; margin-bottom: 0\.8rem; font-weight: 600; color: #4A3728;">\s+<span style="font-size: 1rem;">Upload Video \(MP4\)</span>\s+<span style="font-size: 0\.7rem; color: #8E6E53; opacity: 0\.6;">Max\s+25MB</span>\s+</label>', '', content, flags=re.DOTALL)

# Replace the old upload box div
content = re.sub(r'<div\s+style="position: relative; padding: 1\.5rem; background: #fafafa; border: 2px dashed rgba\(212, 163, 115, 0\.2\); border-radius: 20px; transition: all 0\.3s; text-align: center;">.*?</div>\s+</div>', '', content, flags=re.DOTALL)
# Wait, this is getting messy.

# Safer: replace the whole section by matching unique strings.
start_marker = '<div class="form-group" style="margin-top: 2.5rem;">'
# We just added this.

new_content = """                                         <div class="video-upload-box" onclick="this.querySelector('input').click()">
                                             <input type="file" name="background_video" accept="video/mp4" style="display: none;" onchange="updateVideoFileName(this)">
                                             
                                             <div style="color: #cbd5e1; margin-bottom: 1rem;">
                                                 <svg width="48" height="48" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round" style="color: #D4A373; opacity: 0.6;">
                                                    <path d="M23 7l-7 5 7 5V7z"></path>
                                                    <rect x="1" y="5" width="15" height="14" rx="2" ry="2"></rect>
                                                 </svg>
                                             </div>
                                             
                                             <p style="font-size: 0.95rem; color: #475569; margin: 0; font-weight: 600;">Klik untuk pilih file MP4</p>
                                             
                                             <div id="video-file-pill" class="video-pill" style="{% if invitation.background_video_url.is_empty() %}display: none;{% endif %}">
                                                <div style="width: 8px; height: 8px; background: #10b981; border-radius: 50%;"></div>
                                                <span id="video-filename" style="font-size: 0.8rem; color: #065f46; font-weight: 700; max-width: 250px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap;">
                                                    {% if !invitation.background_video_url.is_empty() %}{{ invitation.background_video_url|split_last("/") }}{% endif %}
                                                </span>
                                             </div>
                                         </div>

                                         <!-- Video Focal Point Slider -->
                                         <div class="form-group" style="margin-top: 3rem;">
                                             <div style="display: flex; justify-content: space-between; align-items: flex-end; margin-bottom: 1.2rem;">
                                                 <h4 class="serif" style="color: #4A3728; font-size: 1.3rem; margin: 0;">Video Vertical Focal Point</h4>
                                                 <span id="focal-val" style="font-size: 0.9rem; color: #94a3b8; font-weight: 700; font-family: 'Inter', sans-serif;">{{ invitation.hero_video_position }}%</span>
                                             </div>

                                             <!-- REALTIME PREVIEW BOX -->
                                             <div class="video-preview-container">
                                                 {% if !invitation.background_video_url.is_empty() %}
                                                 <video id="focal-preview-vid" autoplay muted loop playsinline
                                                     style="width: 100%; height: 100%; object-fit: cover; object-position: center {{ invitation.hero_video_position }}%;">
                                                     <source src="{{ invitation.background_video_url }}" type="video/mp4">
                                                 </video>
                                                 {% else %}
                                                 <div style="width: 100%; height: 100%; display: flex; align-items: center; justify-content: center; color: #64748b; font-size: 0.85rem; text-align: center; padding: 2rem; background: #f1f5f9;">
                                                     Preview akan muncul setelah Anda mengunggah video.
                                                 </div>
                                                 {% endif %}
                                                 
                                                 <div class="focal-line" id="focal-indicator-line" style="top: {{ invitation.hero_video_position }}%;"></div>
                                                 <div class="preview-badge">Preview</div>
                                             </div>

                                             <input type="range" name="hero_video_position" min="0" max="100"
                                                 value="{{ invitation.hero_video_position }}" class="blue-slider"
                                                 oninput="updateFocalPreview(this.value)">
                                             
                                             <div style="display: flex; justify-content: space-between; margin-top: 5px; font-size: 0.7rem; color: #a0aec0; font-weight: 600; text-transform: uppercase; letter-spacing: 0.5px;">
                                                <span>Atas (Top)</span>
                                                <span>Bawah (Bottom)</span>
                                             </div>

                                             <script>
                                                 function updateFocalPreview(val) {
                                                     document.getElementById('focal-val').innerText = val + '%';
                                                     const vid = document.getElementById('focal-preview-vid');
                                                     const line = document.getElementById('focal-indicator-line');
                                                     if (vid) {
                                                         vid.style.objectPosition = `center ${val}%`;
                                                     }
                                                     if (line) {
                                                         line.style.top = val + '%';
                                                     }
                                                 }
                                                 
                                                 function updateVideoFileName(input) {
                                                     if (input.files && input.files[0]) {
                                                         const pill = document.getElementById('video-file-pill');
                                                         const nameSpan = document.getElementById('video-filename');
                                                         nameSpan.innerText = input.files[0].name;
                                                         pill.style.display = 'inline-flex';
                                                     }
                                                 }
                                             </script>
                                         </div>"""

# Find the insertion point
insertion_point = content.find(start_marker) + len(start_marker)
# Find the end of the section to remove (which is the closing </div> of the focal point section)
# It's followed by "Bawah (Bottom)</span>" and then "</div>" and then "</div>" and then "</div>" (end of grid)

# Actually, I'll just replace everything from the old label to the end of the focal point script/div.
end_marker = '<span>Atas (Top)</span>\s+<span>Bawah (Bottom)</span>\s+</div>\s+</div>'

# Let's just find the closing </div> of the outer form-group.
# The section starts with <div class="form-group" style="margin-top: 2.5rem;"> (which we just added)
# Then it has the old label, old box, old focal point label, old focal point preview, old focal point slider, old labels, old script, and old closing </div>.

# I'll use a simpler approach: replace the old label to the old script end.
import re
pattern = re.compile(r'<label.*?updateFocalPreview\(val\).*?</script>\s+</div>', re.DOTALL)
content = pattern.sub(new_content, content)

with open(path, 'w') as f:
    f.write(content)

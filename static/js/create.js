    function addGiftAccount() {
        const container = document.getElementById('gift-accounts-container');
        const newItem = document.createElement('div');
        newItem.className = 'gift-account-item';
        newItem.style.cssText = 'display: flex; flex-direction: column; gap: 1.2rem; margin-bottom: 1.5rem; padding: 1.5rem; background: white; border-radius: 20px; border: 1px solid #eef0f7; position: relative; box-shadow: 0 4px 12px rgba(0,0,0,0.03);';
        newItem.innerHTML = `
            <div style="display: grid; grid-template-columns: 1fr 1fr; gap: 1rem;">
                <div class="input-group">
                    <label style="font-size: 0.75rem; color: #888; text-transform: uppercase; letter-spacing: 0.5px;">Bank/E-Wallet</label>
                    <input type="text" name="bank_name[]" placeholder="BCA, Mandiri, Dana, dll">
                </div>
                <div class="input-group">
                    <label style="font-size: 0.75rem; color: #888; text-transform: uppercase; letter-spacing: 0.5px;">Nomor Rekening</label>
                    <input type="text" name="account_number[]" placeholder="8210370xxx">
                </div>
            </div>
            <div class="input-group">
                <label style="font-size: 0.75rem; color: #888; text-transform: uppercase; letter-spacing: 0.5px;">Atas Nama</label>
                <input type="text" name="account_holder[]" placeholder="Nama Lengkap Pemilik">
            </div>
            <button type="button" onclick="this.parentElement.remove()" style="position: absolute; top: -10px; right: -10px; width: 28px; height: 28px; border-radius: 50%; background: #ff4757; color: white; border: none; cursor: pointer; display: flex; align-items: center; justify-content: center; box-shadow: 0 4px 8px rgba(255,71,87,0.3);"><i class="fas fa-times" style="font-size: 0.7rem;"></i></button>
        `;
        container.appendChild(newItem);
    }

    // --- NEW: Love Story builder ---
    function addStory() {
        const container = document.getElementById('story_container');
        const div = document.createElement('div');
        div.className = 'story-item glass';
        div.style = 'padding: 2rem; border-radius: 25px; background: white; position: relative; border: 1px solid rgba(0,0,0,0.05); box-shadow: 0 10px 30px rgba(0,0,0,0.03); margin-bottom: 2rem;';
        div.innerHTML = `
            <button type="button" onclick="this.parentElement.remove()" style="position: absolute; top: 15px; right: 15px; background: #fef2f2; border: none; color: #ef4444; cursor: pointer; font-size: 0.9rem; width: 30px; height: 30px; border-radius: 50%; display: flex; align-items: center; justify-content: center; transition: all 0.2s;">&times;</button>
            
            <div style="display: grid; grid-template-columns: 200px 1fr; gap: 2rem;">
                <div style="display: flex; flex-direction: column; gap: 12px;">
                    <div style="position: relative; width: 200px; height: 120px; background: #f1f5f9; border-radius: 18px; overflow: hidden; border: 2px dashed #cbd5e1; display: flex; align-items: center; justify-content: center; color: #94a3b8; transition: all 0.3s;">
                        <div style="text-align: center;">
                            <i class="fas fa-image" style="font-size: 1.5rem; margin-bottom: 5px; display: block;"></i>
                            <span style="font-size: 0.7rem; font-weight: 600;">Thumbnail</span>
                        </div>
                        <input type="hidden" name="story_image_url[]" value="">
                    </div>
                    <div style="position: relative;">
                        <input type="file" name="story_image_file[]" accept="image/*" style="position: absolute; inset: 0; opacity: 0; cursor: pointer; width: 100%;" onchange="this.nextElementSibling.innerText = '✓ Terpilih'">
                        <div style="background: #f8fafc; border: 1px solid #e2e8f0; padding: 8px; border-radius: 10px; font-size: 0.7rem; font-weight: 700; text-align: center; color: #64748b;">
                            <i class="fas fa-upload" style="margin-right: 5px;"></i> Unggah Foto
                        </div>
                    </div>
                </div>
                <div style="display: flex; flex-direction: column; gap: 15px;">
                    <div class="form-group mb-0">
                        <label style="font-size: 0.7rem; font-weight: 800; color: #8E6E53; text-transform: uppercase; letter-spacing: 1px; margin-bottom: 5px;">Judul Cerita</label>
                        <input type="text" name="story_title[]" value="" placeholder="Contoh: Awal Pertemuan" style="font-weight: 700; font-size: 1rem; border: none; background: #f8fafc; padding: 12px 15px; border-radius: 12px; width: 100%;">
                    </div>
                    <div class="form-group mb-0">
                        <label style="font-size: 0.7rem; font-weight: 800; color: #8E6E53; text-transform: uppercase; letter-spacing: 1px; margin-bottom: 5px;">Tanggal / Waktu</label>
                        <input type="text" name="story_date[]" value="" placeholder="Contoh: Juni 2025" style="font-weight: 600; font-size: 0.9rem; border: none; background: #f8fafc; padding: 12px 15px; border-radius: 12px; width: 100%;">
                    </div>
                    <div class="form-group mb-0">
                        <label style="font-size: 0.7rem; font-weight: 800; color: #8E6E53; text-transform: uppercase; letter-spacing: 1px; margin-bottom: 5px;">Deskripsi Cerita</label>
                        <textarea name="story_description[]" rows="3" placeholder="Ceritakan momen indah ini..." style="border: none; background: #f8fafc; padding: 12px 15px; border-radius: 12px; width: 100%; font-size: 0.9rem; line-height: 1.5;"></textarea>
                    </div>
                </div>
            </div>
        `;
        container.appendChild(div);
        div.scrollIntoView({ behavior: 'smooth', block: 'center' });
    }

    // --- NEW: Custom MP3 Playlist files multi-upload and alignment ---
    let selectedPlaylistSongs = [];
    function handlePlaylistChange(input) {
        if (input.files && input.files.length > 0) {
            const list = document.getElementById('playlist-list');
            for (let file of input.files) {
                selectedPlaylistSongs.push(file);
                const item = document.createElement('div');
                item.className = 'song-item';
                item.style = 'display: flex; justify-content: space-between; align-items: center; background: #f8fafc; padding: 12px 18px; border-radius: 12px; border: 1px solid #e2e8f0; margin-bottom: 8px;';
                item.innerHTML = `
                    <div style="display: flex; align-items: center; gap: 8px;">
                        <span style="font-size: 1.2rem; color: #D4A373;">🎵</span>
                        <span style="font-size: 0.85rem; font-weight: 700; color: #4A3728; max-width: 250px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap;">${file.name}</span>
                    </div>
                    <button type="button" onclick="removePlaylistSong(this, '${file.name}')" style="background: none; border: none; color: #ef4444; font-weight: 800; cursor: pointer; font-size: 1.1rem;">&times;</button>
                `;
                list.appendChild(item);
            }
            syncPlaylistInput();
        }
    }

    function removePlaylistSong(btn, filename) {
        selectedPlaylistSongs = selectedPlaylistSongs.filter(f => f.name !== filename);
        btn.parentElement.remove();
        syncPlaylistInput();
    }

    function syncPlaylistInput() {
        const input = document.getElementById('playlist-upload');
        const dt = new DataTransfer();
        for (let file of selectedPlaylistSongs) {
            dt.items.add(file);
        }
        input.files = dt.files;
    }

    // --- NEW: Video Background focal preview alignment ---
    function updateFocalPreview(val) {
        const display = document.getElementById('focal-val');
        if (display) display.innerText = val + '%';
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

            // Realtime preview update
            const vid = document.getElementById('focal-preview-vid');
            const noPreview = document.getElementById('no-video-preview');
            if (vid && noPreview) {
                const objectUrl = URL.createObjectURL(input.files[0]);
                vid.src = objectUrl;
                vid.style.display = 'block';
                noPreview.style.display = 'none';
            }
        }
    }

    // --- NEW: Photo Gallery multi-upload & deletion sync ---
    let selectedGalleryPhotos = [];
    function handleGalleryUpload(files) {
        if (files && files.length > 0) {
            const grid = document.getElementById('gallery-grid');
            for (let file of files) {
                selectedGalleryPhotos.push(file);

                const item = document.createElement('div');
                item.className = 'gallery-photo-preview';
                item.style = 'position: relative; aspect-ratio: 1; border-radius: 12px; overflow: hidden; border: 1px solid #cbd5e1;';

                const reader = new FileReader();
                reader.onload = function (e) {
                    item.innerHTML = `
                        <img src="${e.target.result}" style="width: 100%; height: 100%; object-fit: cover;">
                        <button type="button" onclick="removeGalleryPhoto(this, '${file.name}')" style="position: absolute; top: 5px; right: 5px; background: rgba(239, 68, 68, 0.9); border: none; color: white; cursor: pointer; font-size: 0.8rem; width: 22px; height: 22px; border-radius: 50%; display: flex; align-items: center; justify-content: center;">&times;</button>
                    `;
                }
                reader.readAsDataURL(file);
                grid.appendChild(item);
            }
            syncGalleryInput();
        }
    }

    function removeGalleryPhoto(btn, filename) {
        selectedGalleryPhotos = selectedGalleryPhotos.filter(f => f.name !== filename);
        btn.parentElement.remove();
        syncGalleryInput();
    }

    function syncGalleryInput() {
        const input = document.getElementById('gallery_file_input');
        const dt = new DataTransfer();
        for (let file of selectedGalleryPhotos) {
            dt.items.add(file);
        }
        input.files = dt.files;
    }

    // --- NEW: Video Gallery multi-upload & deletion sync ---
    let selectedGalleryVideos = [];
    function handleVideoGalleryUpload(files) {
        if (files && files.length > 0) {
            const grid = document.getElementById('video-gallery-grid');
            for (let file of files) {
                selectedGalleryVideos.push(file);

                const item = document.createElement('div');
                item.className = 'gallery-video-preview';
                item.style = 'position: relative; aspect-ratio: 16/9; border-radius: 12px; overflow: hidden; border: 1px solid #cbd5e1; background: black;';

                const objectUrl = URL.createObjectURL(file);
                item.innerHTML = `
                    <video src="${objectUrl}" style="width: 100%; height: 100%; object-fit: cover;" autoplay muted loop playsinline></video>
                    <button type="button" onclick="removeGalleryVideo(this, '${file.name}')" style="position: absolute; top: 5px; right: 5px; background: rgba(239, 68, 68, 0.9); border: none; color: white; cursor: pointer; font-size: 0.8rem; width: 22px; height: 22px; border-radius: 50%; display: flex; align-items: center; justify-content: center;">&times;</button>
                `;
                grid.appendChild(item);
            }
            syncVideoGalleryInput();
        }
    }

    function removeGalleryVideo(btn, filename) {
        selectedGalleryVideos = selectedGalleryVideos.filter(f => f.name !== filename);
        btn.parentElement.remove();
        syncVideoGalleryInput();
    }

    function syncVideoGalleryInput() {
        const input = document.getElementById('video_file_input');
        const dt = new DataTransfer();
        for (let file of selectedGalleryVideos) {
            dt.items.add(file);
        }
        input.files = dt.files;
    }

    let isSlugManuallyEdited = false;

    let currentStep = 1;
    let currentCategory = 'all';
    let templateLimit = 3; // Default for NOBLE

    let isSlugAvailable = true;

    function nextStep(step) {
        if (document.startViewTransition) {
            document.startViewTransition(() => _nextStepInternal(step));
        } else {
            _nextStepInternal(step);
        }
    }

    function _nextStepInternal(step) {
        // Validation for moving from step 1 to 2
        const currentStep = Array.from(document.querySelectorAll('.form-step')).findIndex(s => s.style.display !== 'none') + 1;

        if (currentStep === 1 && step === 2 && !isSlugAvailable) {
            Swal.fire({
                icon: 'error',
                title: 'URL Slug Tidak Tersedia',
                text: 'Silakan gunakan slug lain atau gunakan saran alternatif yang tersedia.',
                confirmButtonColor: '#8E6E53'
            });
            return;
        }

        console.log("Navigating to step:", step);
        if (step === 8) {
            loadLivePreview();
        }

        const steps = document.querySelectorAll('.form-step');
        steps.forEach(s => s.style.display = 'none');

        const targetStep = document.getElementById('step-' + step);
        if (targetStep) {
            targetStep.style.display = 'block';
        } else {
            console.error("Target step not found: step-" + step);
        }

        document.querySelectorAll('.step-item').forEach((i, idx) => {
            const stepNum = idx + 1;
            i.classList.remove('active', 'completed', 'adjacent');
            if (stepNum < step) {
                i.classList.add('completed');
            } else if (stepNum === step) {
                i.classList.add('active');
            }
            if (stepNum === step - 1 || stepNum === step + 1) {
                i.classList.add('adjacent');
            }
        });

        // Smoothly auto-scroll active step to center of progress tracker container
        const activeIndicator = document.getElementById('step-' + step + '-indicator');
        if (activeIndicator) {
            activeIndicator.scrollIntoView({ behavior: 'smooth', block: 'nearest', inline: 'center' });
        }

        try {
            updatePreview();
        } catch (e) {
            console.warn("updatePreview failed:", e);
        }

        window.scrollTo({ top: 150, behavior: 'smooth' });
    }

    async function loadLivePreview(overrideTemplate = null) {
        const loader = document.getElementById('preview-loader');
        const iframe = document.getElementById('live-preview-iframe');
        loader.style.display = 'flex';

        const selectedTemplates = Array.from(document.querySelectorAll('input[name="template_name"]:checked')).map(cb => cb.value);

        // Initial load should populate tabs
        if (!overrideTemplate) updatePreviewTabs(selectedTemplates);

        const formData = {
            template_name: overrideTemplate || selectedTemplates[0] || (Object.keys(TPL_MAP)[0] || "trendvibe"),
            selected_templates: selectedTemplates,
            couple_name_short: document.getElementById('in-name').value,
            bride_name: document.getElementById('in-bride-name').value,
            bride_full_name: document.querySelector('input[name="bride_full_name"]').value,
            groom_name: document.getElementById('in-groom-name').value,
            groom_full_name: document.querySelector('input[name="groom_full_name"]').value,
            bride_father: document.querySelector('input[name="bride_father"]').value || "-",
            bride_mother: document.querySelector('input[name="bride_mother"]').value || "-",
            groom_father: document.querySelector('input[name="groom_father"]').value || "-",
            groom_mother: document.querySelector('input[name="groom_mother"]').value || "-",
            ceremony_date: document.getElementById('in-date').value,
            ceremony_time: document.getElementById('in-time').value,
            ceremony_venue: document.getElementById('in-venue').value,
            ceremony_address: document.getElementById('in-address').value,
            ceremony_maps: document.getElementById('in-maps').value,
            reception_date: document.getElementById('in-reception-date')?.value || document.getElementById('in-date').value,
            reception_time: document.getElementById('in-reception-time')?.value || "",
            reception_venue: document.getElementById('in-reception-venue')?.value || "",
            reception_address: document.getElementById('in-reception-address')?.value || "",
            reception_maps: document.getElementById('in-reception-maps')?.value || "",
            quote_text: document.getElementById('in-quote').value,
            quote_source: document.getElementById('in-quote-source').value,
        };

        try {
            const response = await fetch('/api/preview', {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify(formData)
            });
            const html = await response.text();

            const doc = iframe.contentWindow.document;
            doc.open();
            doc.write(html);
            doc.close();

            setTimeout(() => { loader.style.display = 'none'; }, 500);
        } catch (error) {
            console.error('Preview failed:', error);
        }
    }

    function updatePreviewTabs(selected) {
        const container = document.getElementById('preview-template-tabs');
        if (!container) return;

        const tplMap = TPL_MAP;

        container.innerHTML = selected.map((val, idx) => {
            const name = tplMap[val] || val;
            const activeClass = idx === 0 ? 'active' : '';
            return `<button type="button" class="mini-filter-chip ${activeClass}" onclick="switchPreviewTemplate('${val}', this)" style="font-size: 0.7rem; padding: 6px 12px; margin: 0;">${name}</button>`;
        }).join('');
    }

    function switchPreviewTemplate(val, btn) {
        document.querySelectorAll('#preview-template-tabs .mini-filter-chip').forEach(b => b.classList.remove('active'));
        btn.classList.add('active');
        loadLivePreview(val);
    }

    function updateSelectedPlan(name, price, limit) {
        // Update price display in Step 5
        const priceDisplay = document.getElementById('pv-total-final');
        if (priceDisplay) {
            priceDisplay.textContent = `Rp ${price.toLocaleString('id-ID')}`;
            priceDisplay.dataset.basePrice = price;
            
            // Reapply promo code if exists
            const promoInput = document.getElementById('in-promo-code');
            if (promoInput && promoInput.value) {
                promoInput.dispatchEvent(new Event('input', { bubbles: true }));
            }
        }

        // Update Plan Name in Review (Step 5)
        const planDisplay = document.querySelector('.plan-selection-name');
        if (planDisplay) {
            planDisplay.textContent = name + " Collection";
        }

        // Update template limit
        templateLimit = limit || 3;

        validateTemplateSelection(); // Re-validate with new limit
    }

    function validateTemplateSelection() {
        const selected = document.querySelectorAll('input[name="template_name"]:checked');

        // Update summary badge/bar
        const countBadge = document.getElementById('selection-count-badge');
        const limitStatus = document.getElementById('selection-limit-status');
        const labelsContainer = document.getElementById('selection-labels-badge');
        const summaryBar = document.getElementById('selection-summary-bar');

        if (countBadge) countBadge.innerText = selected.length;
        if (limitStatus) {
            const currentPlan = document.querySelector('input[name="plan_name"]:checked')?.value || 'NOBLE';
            const planNames = {
                'NOBLE': 'NOBLE',
                'ROYAL': 'ROYAL',
                'DYNASTY': 'DYNASTY'
            };
            const planNameDisplay = planNames[currentPlan] || currentPlan;
            limitStatus.innerText = `${selected.length} / ${templateLimit === 999 ? '∞' : templateLimit} ${planNameDisplay} SELECTED`;
        }

        if (labelsContainer) {
            const tplMap = TPL_MAP;

            labelsContainer.innerHTML = Array.from(selected).map(el => {
                const name = tplMap[el.value] || el.value;
                return `<span style="background: #fdf6f0; color: #8E6E53; font-size: 0.65rem; padding: 4px 10px; border-radius: 20px; font-weight: 700; border: 1px solid rgba(142,110,83,0.1);">${name}</span>`;
            }).join('');
        }

        // Visual feedback for the bar
        if (summaryBar) {
            if (selected.length > 0) {
                summaryBar.style.opacity = '1';
                summaryBar.style.transform = 'translateY(0)';
            } else {
                summaryBar.style.opacity = '0.5';
            }
        }

        if (selected.length > templateLimit) {
            let msg = `Paket ini maksimal ${templateLimit} template.`;

            Swal.fire({
                icon: 'warning',
                title: 'Limit Tercapai',
                text: msg + ' Silakan upgrade paket untuk memilih lebih banyak.',
                confirmButtonColor: '#8E6E53'
            });

            // If called from an event, uncheck the target
            if (window.event && window.event.target && window.event.target.name === 'template_name') {
                window.event.target.checked = false;
            } else {
                // If called from updateSelectedPlan, we might need to uncheck extras
                for (let i = templateLimit; i < selected.length; i++) {
                    selected[i].checked = false;
                }
            }
            // Recursive call to update the UI again after unchecking
            validateTemplateSelection();
            return;
        }
        updatePreview();
    }

    function setCategoryFilter(category, btn) {
        currentCategory = category;
        document.querySelectorAll('.mini-filter-chip').forEach(b => b.classList.remove('active'));
        btn.classList.add('active');
        filterTemplates();
    }

    function filterTemplates() {
        const query = document.getElementById('template-search').value.toLowerCase();
        const cards = document.querySelectorAll('.tpl-card');

        cards.forEach(card => {
            const title = card.querySelector('.serif').innerText.toLowerCase();
            const desc = card.querySelector('p').innerText.toLowerCase();
            const category = card.getAttribute('data-category');

            const matchesSearch = title.includes(query) || desc.includes(query);
            const matchesCategory = currentCategory === 'all' || category === currentCategory;

            if (matchesSearch && matchesCategory) {
                card.style.display = 'block';
            } else {
                card.style.display = 'none';
            }
        });
    }

    let slugCheckTimeout = null;

    function checkSlugAvailability() {
        const slugInput = document.getElementById('in-slug');
        const statusEl = document.getElementById('slug-status');
        const slug = slugInput.value.trim();

        if (!slug) {
            statusEl.innerText = '';
            return;
        }

        if (slugCheckTimeout) clearTimeout(slugCheckTimeout);

        statusEl.innerHTML = '<span style="color: #999; font-style: italic;">Checking availability...</span>';

        slugCheckTimeout = setTimeout(async () => {
            try {
                const res = await fetch(`/api/check-slug/${encodeURIComponent(slug)}`);
                const data = await res.json();

                if (data.available) {
                    isSlugAvailable = true;
                    statusEl.innerHTML = `<span style="color: #2e7d32; font-weight: 700;">✅ ${data.message}</span>`;
                } else {
                    isSlugAvailable = false;
                    const randomSuffix = Math.floor(100 + Math.random() * 900);
                    const alternative = `${slug}-${randomSuffix}`;
                    statusEl.innerHTML = `<span style="color: #c62828; font-weight: 700;">❌ ${data.message}.</span> <br><span style="color: #666; font-size: 0.7rem;">Try: <a href="#" onclick="applySlug('${alternative}'); return false;" style="color: #8E6E53; font-weight: 800; text-decoration: underline;">${alternative}</a></span>`;
                }
            } catch (e) {
                console.error("Failed to check slug", e);
                statusEl.innerText = '';
            }
        }, 600);
    }

    function applySlug(slug) {
        const slugInput = document.getElementById('in-slug');
        if (slugInput) {
            slugInput.value = slug;
            isSlugManuallyEdited = true;
            checkSlugAvailability();
        }
    }

    function autoSlug() {
        if (isSlugManuallyEdited) return;
        const name = document.getElementById('in-name').value;
        const slugInput = document.getElementById('in-slug');
        if (slugInput) {
            slugInput.value = name.toLowerCase()
                .replace(/&/g, 'and')
                .replace(/[^a-z0-9 ]/g, '')
                .trim()
                .replace(/\s+/g, '-');
            checkSlugAvailability();
        }
    }

    function prevStep(step) {
        nextStep(step);
    }

    function formatDate(dateStr) {
        if (!dateStr || !dateStr.includes('-')) return dateStr;
        const parts = dateStr.split('-');
        if (parts.length !== 3) return dateStr;
        const months = ["Januari", "Februari", "Maret", "April", "Mei", "Juni", "Juli", "Agustus", "September", "Oktober", "November", "Desember"];
        const year = parts[0];
        const monthNum = parseInt(parts[1]) - 1;
        const day = parseInt(parts[2]);
        return `${day} ${months[monthNum]} ${year}`;
    }

    function updatePreview() {
        console.log("Updating preview...");
        // Elements to update (Using optional chaining and new classes)
        const elName = document.getElementById('pv-name');
        const elSlug = document.getElementById('pv-slug');
        const elPreviewLink = document.getElementById('pv-preview-link');
        const elDate = document.getElementById('pv-date');
        const elVenue = document.getElementById('pv-venue');
        const elPlan = document.querySelector('.plan-selection-name'); // New class
        const elTotal = document.getElementById('pv-total-final');

        // Inputs
        const inName = document.getElementById('in-name')?.value || 'Romeo & Julia';
        const inSlug = document.getElementById('in-slug')?.value || 'url-slug';
        const inDate = document.getElementById('in-date')?.value || 'Wedding Date';
        const inVenue = document.getElementById('in-venue')?.value || 'Event Venue';
        const inAddress = document.getElementById('in-address')?.value || '';

        // Apply Updates
        if (elName) elName.innerText = inName;
        if (elSlug) elSlug.innerText = 'Main URL: /invitation/' + inSlug;
        if (elPreviewLink) {
            elPreviewLink.innerText = '/preview/' + inSlug;
            elPreviewLink.href = '/preview/' + inSlug;
        }
        if (elDate) elDate.innerText = formatDate(inDate);
        if (elVenue) elVenue.innerText = inAddress ? inVenue + " (" + inAddress + ")" : inVenue;

        // Template logic
        const selectedTplElements = document.querySelectorAll('input[name="template_name"]:checked');
        const elTplSummary = document.querySelector('.selected-templates-list');

        if (elTplSummary) {
            const tplMap = TPL_MAP;

            const names = Array.from(selectedTplElements).map(el => tplMap[el.value] || el.value);
            elTplSummary.innerHTML = names.map(name => `<div style="margin-bottom: 2px;">• ${name}</div>`).join('');

            // Also update the single name display if it still exists
            const elTplName = document.querySelector('.tpl-selection-name');
            if (elTplName) elTplName.innerText = names.join(', ');
        }

        // Plan logic
        const selectedPlanElement = document.querySelector('input[name="plan_name"]:checked');

        let planDisplay = "Noble Collection";
        let priceDisplay = "Rp 50.000";
        let basePrice = "50000";

        if (selectedPlanElement) {
            const planVal = selectedPlanElement.value;
            if (planVal === 'ROYAL') {
                planDisplay = "Royal Collection";
                priceDisplay = "Rp 100.000";
                basePrice = "100000";
            } else if (planVal === 'DYNASTY') {
                planDisplay = "Dynasty Collection";
                priceDisplay = "Rp 300.000";
                basePrice = "300000";
            }
        }

        if (elPlan) elPlan.innerText = planDisplay;
        if (elTotal) {
            elTotal.innerText = priceDisplay;
            elTotal.setAttribute('data-base-price', basePrice);
        }
    }

    function handleFileSelect(input) {
        const zone = input.parentElement;
        const display = zone.querySelector('.file-name-display');
        const text = zone.querySelector('.upload-text');

        if (input.files && input.files.length > 0) {
            let name = input.files[0].name;
            if (input.files.length > 1) name = input.files.length + " files selected";

            display.innerText = "✓ " + name;
            display.style.display = 'block';
            text.style.display = 'none';
            zone.style.borderColor = 'var(--color-accent-rose)';
            zone.style.background = '#fff9f9';
        }
    }

    function toggleReception() {
        const section = document.getElementById('reception-section');
        const isChecked = document.getElementById('same-as-ceremony').checked;
        section.style.display = isChecked ? 'none' : 'block';
        if (isChecked) syncReception();
    }

    function syncReception() {
        if (document.getElementById('same-as-ceremony').checked) {
            document.getElementById('in-reception-venue').value = document.getElementById('in-venue').value;
            document.getElementById('in-reception-address').value = document.getElementById('in-address').value;
            document.getElementById('in-reception-maps').value = document.getElementById('in-maps').value;
            document.getElementById('in-reception-date').value = document.getElementById('in-date').value;
            document.getElementById('in-reception-time').value = document.getElementById('in-time').value;
        }
    }

    // AI Copywriter Implementation
    async function generateAiQuote() {
        const btn = document.querySelector('.btn-ai-magic');
        const quoteTextarea = document.getElementById('in-quote');
        const quoteSourceInput = document.getElementById('in-quote-source');

        if (btn.classList.contains('loading')) return;

        // Get context
        const bride = document.getElementById('in-bride-name').value || "Nazma";
        const groom = document.getElementById('in-groom-name').value || "Guntur";

        btn.classList.add('loading');
        btn.querySelector('span:not(.wand-icon)').innerText = "Thinking...";
        showAiToast("✨ AI is crafting your message...");

        try {
            const response = await fetch('/api/ai/generate-text', {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({
                    prompt: `Write a short, beautiful, and romantic wedding quote for ${bride} & ${groom}. It can be a poetic sentence or an Islamic-inspired quote about marriage. Keep it in Indonesian language (Bahasa Indonesia). The quote should be elegant. Format: Quote Text ### Quote Source`,
                })
            });

            if (!response.ok) throw new Error('AI Service Unavailable');

            const data = await response.json();
            const [text, source] = data.text.split('###').map(s => s.trim());

            // Typewriter effect or direct set
            quoteTextarea.value = text || data.text;
            if (source) quoteSourceInput.value = source;

            showAiToast("✨ Masterpiece generated!");
        } catch (err) {
            console.error(err);
            showAiToast("❌ Oops! AI was interrupted.");
        } finally {
            btn.classList.remove('loading');
            btn.querySelector('span:not(.wand-icon)').innerText = "Generate AI Quote";
        }
    }

    function showAiToast(msg) {
        let toast = document.getElementById('ai-toast');
        if (!toast) {
            toast = document.createElement('div');
            toast.id = 'ai-toast';
            toast.className = 'ai-toast';
            document.body.appendChild(toast);
        }
        toast.innerText = msg;
        toast.classList.add('show');
        setTimeout(() => toast.classList.remove('show'), 3000);
    }

    async function translateWithAi() {
        const lang = document.getElementById('ai-target-lang').value;
        const quoteTextarea = document.getElementById('in-quote');
        const currentText = quoteTextarea.value;

        if (!currentText) {
            showAiToast("⚠️ Please fill in your quote first.");
            return;
        }

        showAiToast(`🌍 Translating to ${lang}...`);

        try {
            const response = await fetch('/api/ai/generate-text', {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({
                    prompt: `Translate the following wedding invitation quote to ${lang}. Keep the tone romantic and formal: "${currentText}"`,
                })
            });

            const data = await response.json();
            quoteTextarea.value = data.text;
            updatePreview();
            showAiToast("✨ Translation complete!");
        } catch (err) {
            showAiToast("❌ Translation failed.");
        }
    }

    async function enhancePhotos() {
        const input = document.querySelector('input[name="gallery[]"]');
        if (!input.files || input.files.length === 0) {
            showAiToast("⚠️ Please upload photos first.");
            return;
        }

        showAiToast("✨ AI is analyzing and enhancing your photos...");

        // Simulating enhancement
        setTimeout(() => {
            showAiToast("✅ All photos enhanced for premium display!");
        }, 3000);
    }

    // Initialize from URL params or state
    window.onload = function () {
        console.log("Window loaded, initializing...");
        try {
            const urlParams = new URLSearchParams(window.location.search);

            // Auto-select template
            const tpl = urlParams.get('template');
            if (tpl) {
                const radio = document.querySelector(`input[name="template_name"][value="${tpl}"]`);
                if (radio) radio.checked = true;
            }

            // Auto-select plan
            const plan = urlParams.get('plan');
            if (plan) {
                const radio = document.querySelector(`input[name="plan_name"][value="${plan}"]`);
                if (radio) {
                    radio.checked = true;
                    const priceMap = { 'NOBLE': 50000, 'ROYAL': 100000, 'DYNASTY': 300000 };
                    updateSelectedPlan(plan, priceMap[plan] || 50000);
                }
            }

            // Set initial wedding date to one month after today if not set
            const inDateInput = document.getElementById('in-date');
            if (inDateInput && !inDateInput.value) {
                const today = new Date();
                today.setMonth(today.getMonth() + 1);
                const year = today.getFullYear();
                const month = String(today.getMonth() + 1).padStart(2, '0');
                const day = String(today.getDate()).padStart(2, '0');
                inDateInput.value = `${year}-${month}-${day}`;
            }

            validateTemplateSelection();
            updatePreview();
            toggleReception();
        } catch (e) {
            console.error("Initialization failed:", e);
        }
    }

    function openAiAssistant() {
        document.getElementById('aiAssistantModal').style.display = 'flex';
    }

    function closeAiAssistant() {
        document.getElementById('aiAssistantModal').style.display = 'none';
    }

    // --- AI Session Logic ---
    let currentAiSessionId = localStorage.getItem('ai_session_id');

    document.addEventListener('DOMContentLoaded', async () => {
        if (currentAiSessionId) {
            try {
                const res = await fetch(`/api/ai/session/${currentAiSessionId}`);
                if (res.ok) {
                    const session = await res.json();
                    restoreAiSession(session);
                }
            } catch (err) {
                console.error("Failed to restore AI session", err);
            }
        }
    });

    function restoreAiSession(session) {
        const chatBody = document.getElementById('aiChatBody');

        // Restore History
        if (session.chat_history && Array.isArray(session.chat_history)) {
            session.chat_history.forEach(msg => {
                if (msg.role === 'system') return;
                appendMessageToChat(msg.role, msg.content);
            });
        }

        // Restore Form State
        if (session.form_state) {
            fillFormFromData(session.form_state);
        }
    }

    function appendMessageToChat(role, content) {
        const chatBody = document.getElementById('aiChatBody');
        const isAi = role === 'assistant' || role === 'ai';
        const wrapper = document.createElement('div');

        if (isAi) {
            wrapper.style.cssText = 'display: flex; gap: 12px; align-self: flex-start; max-width: 85%;';
            wrapper.innerHTML = `
                <div style="width: 32px; height: 32px; background: #eee; border-radius: 50%; display: flex; align-items: center; justify-content: center; font-size: 16px; flex-shrink: 0;">🤖</div>
                <div style="background: white; color: #2A1E14; padding: 1rem 1.2rem; border-radius: 20px; border-top-left-radius: 4px; line-height: 1.6; font-size: 0.9rem; box-shadow: 0 2px 10px rgba(0,0,0,0.03); border: 1px solid #f0f0f0;">
                    ${content}
                </div>
            `;
        } else {
            wrapper.style.cssText = 'display: flex; gap: 12px; align-self: flex-end; max-width: 85%; flex-direction: row-reverse;';
            wrapper.innerHTML = `
                <div style="width: 32px; height: 32px; background: #2A1E14; color: white; border-radius: 50%; display: flex; align-items: center; justify-content: center; font-size: 14px; flex-shrink: 0; font-weight: 700;">U</div>
                <div style="background: #2A1E14; color: white; padding: 1rem 1.2rem; border-radius: 20px; border-top-right-radius: 4px; line-height: 1.6; font-size: 0.9rem; box-shadow: 0 10px 20px rgba(42,30,20,0.1);">
                    ${content}
                </div>
            `;
        }

        chatBody.appendChild(wrapper);
        chatBody.scrollTop = chatBody.scrollHeight;
    }

    function showTypingIndicator() {
        const chatBody = document.getElementById('aiChatBody');
        const indicator = document.createElement('div');
        indicator.id = 'ai-typing-indicator';
        indicator.style.cssText = 'display: flex; gap: 12px; align-self: flex-start; max-width: 85%; margin-bottom: 1rem;';
        indicator.innerHTML = `
            <div style="width: 32px; height: 32px; background: #eee; border-radius: 50%; display: flex; align-items: center; justify-content: center; font-size: 16px; flex-shrink: 0;">🤖</div>
            <div style="background: white; color: #2A1E14; padding: 1rem 1.5rem; border-radius: 20px; border-top-left-radius: 4px; box-shadow: 0 2px 10px rgba(0,0,0,0.03); border: 1px solid #f0f0f0; display: flex; gap: 5px; align-items: center;">
                <span class="typing-dot"></span>
                <span class="typing-dot" style="animation-delay: 0.2s;"></span>
                <span class="typing-dot" style="animation-delay: 0.4s;"></span>
            </div>
        `;
        chatBody.appendChild(indicator);
        chatBody.scrollTop = chatBody.scrollHeight;
    }

    function hideTypingIndicator() {
        const indicator = document.getElementById('ai-typing-indicator');
        if (indicator) indicator.remove();
    }

    function fillFormFromData(data) {
        const fieldMap = {
            'in-name': data.couple_name_short,
            'in-slug': data.slug,
            'in-date': data.ceremony_date,
            'in-time': data.ceremony_time,
            'in-venue': data.ceremony_venue,
            'in-address': data.ceremony_address,
            'in-maps': data.ceremony_maps,
            'in-bride-name': data.bride_name,
            'in-bride-full-name': data.bride_full_name,
            'in-bride-father': data.bride_father,
            'in-bride-mother': data.bride_mother,
            'in-groom-name': data.groom_name,
            'in-groom-full-name': data.groom_full_name,
            'in-groom-father': data.groom_father,
            'in-groom-mother': data.groom_mother,
            'in-reception-date': data.reception_date,
            'in-reception-time': data.reception_time,
            'in-reception-venue': data.reception_venue,
            'in-reception-address': data.reception_address,
            'in-reception-maps': data.reception_maps,
            'in-quote': data.quote_text,
            'in-quote-source': data.quote_source
        };

        for (const [id, value] of Object.entries(fieldMap)) {
            const el = document.getElementById(id);
            if (el && value && value !== "" && value !== "null") {
                el.value = value;
                el.classList.add('highlight-filled');
                setTimeout(() => el.classList.remove('highlight-filled'), 3000);
            }
        }
        autoSlug();
        syncReception();
        updatePreview();
    }

    async function parseAndFillForm() {
        const inputEl = document.getElementById('aiAssistantInput');
        const prompt = inputEl.value.trim();
        if (!prompt) return;

        appendMessageToChat('user', prompt);
        inputEl.value = '';
        inputEl.style.height = '45px'; // Reset height

        showTypingIndicator();

        try {
            const response = await fetch('/api/ai/parse-form', {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({
                    prompt,
                    session_id: currentAiSessionId
                })
            });

            const rawResult = await response.json();

            // Update session ID if it was newly created
            if (rawResult.session_id) {
                currentAiSessionId = rawResult.session_id;
                localStorage.setItem('ai_session_id', currentAiSessionId);
            }

            let parsed;
            try {
                parsed = JSON.parse(rawResult.text);
            } catch (e) {
                console.error("JSON Parse Error:", e, rawResult.text);
                throw new Error("Invalid AI response format");
            }

            const data = parsed.data || {};

            hideTypingIndicator();
            fillFormFromData(data);
            appendMessageToChat('assistant', parsed.reply || "Data sudah saya proses!");

            if (parsed.missing && parsed.missing.length === 0) {
                showAiToast("🎉 Semua data penting sudah lengkap!");
            }
        } catch (err) {
            console.error("AI Process Error:", err);
            hideTypingIndicator();
            showAiToast("❌ Gagal memproses data. Silakan coba lagi.");
            appendMessageToChat('assistant', "Maaf, saya gagal memproses pesan tersebut. Bisa diulangi dengan kalimat yang berbeda?");
        }
    }

    function handleAiChatFileUpload(input) {
        if (!input.files || input.files.length === 0) return;

        const chatBody = document.getElementById('aiChatBody');
        const file = input.files[0];
        const isVideo = file.type.startsWith('video/');

        // Append User File Message
        const userMsgWrapper = document.createElement('div');
        userMsgWrapper.style.cssText = 'display: flex; gap: 12px; align-self: flex-end; max-width: 85%; flex-direction: row-reverse;';

        const userAvatar = document.createElement('div');
        userAvatar.style.cssText = 'width: 32px; height: 32px; background: #2A1E14; color: white; border-radius: 50%; display: flex; align-items: center; justify-content: center; font-size: 14px; flex-shrink: 0; font-weight: 700;';
        userAvatar.textContent = 'U';

        const userMsg = document.createElement('div');
        userMsg.style.cssText = 'background: #2A1E14; color: white; padding: 0.8rem; border-radius: 20px; border-top-right-radius: 4px; line-height: 1.6; font-size: 0.9rem; box-shadow: 0 10px 20px rgba(42,30,20,0.1);';
        userMsg.innerHTML = `<div style="font-size: 10px; opacity: 0.7; margin-bottom: 5px;">Uploading ${isVideo ? 'Video' : 'Image'}...</div><div style="font-weight: 700;">${file.name}</div>`;

        userMsgWrapper.appendChild(userAvatar);
        userMsgWrapper.appendChild(userMsg);
        chatBody.appendChild(userMsgWrapper);
        chatBody.scrollTop = chatBody.scrollHeight;

        // Sync with main form inputs
        if (isVideo) {
            const videoInput = document.querySelector('input[name="video"]');
            if (videoInput) {
                videoInput.files = input.files;
                // Trigger preview
                handleFileSelect(videoInput);
            }
        } else {
            const galleryInput = document.querySelector('input[name="gallery[]"]');
            if (galleryInput) {
                // For gallery, we usually want to append, but standard file inputs don't append easily.
                // We'll set it as the current selection.
                galleryInput.files = input.files;
                handleFileSelect(galleryInput);
            }
        }

        // AI Reply confirmation
        setTimeout(() => {
            const aiMsgWrapper = document.createElement('div');
            aiMsgWrapper.style.cssText = 'display: flex; gap: 12px; align-self: flex-start; max-width: 85%;';

            const aiAvatar = document.createElement('div');
            aiAvatar.style.cssText = 'width: 32px; height: 32px; background: #eee; border-radius: 50%; display: flex; align-items: center; justify-content: center; font-size: 16px; flex-shrink: 0;';
            aiAvatar.textContent = '🤖';

            const aiMsg = document.createElement('div');
            aiMsg.style.cssText = 'background: white; color: #2A1E14; padding: 1rem 1.2rem; border-radius: 20px; border-top-left-radius: 4px; line-height: 1.6; font-size: 0.9rem; box-shadow: 0 2px 10px rgba(0,0,0,0.03); border: 1px solid #f0f0f0;';
            aiMsg.innerHTML = `Wah, file <b>${file.name}</b> sudah saya terima dan saya masukkan ke ${isVideo ? 'Video Utama' : 'Galeri'}! ✨ Ada lagi yang mau ditambahkan?`;

            aiMsgWrapper.appendChild(aiAvatar);
            aiMsgWrapper.appendChild(aiMsg);
            chatBody.appendChild(aiMsgWrapper);
            chatBody.scrollTop = chatBody.scrollHeight;
        }, 1500);
    }

    // Fix for "Complete Order" button not clickable due to hidden validation
    document.getElementById('multistep-form').addEventListener('submit', function (e) {
        const form = e.target;
        if (!form.checkValidity()) {
            e.preventDefault();

            // Find the first invalid element
            const firstInvalid = form.querySelector(':invalid');
            if (firstInvalid) {
                // Find which step it belongs to
                const step = firstInvalid.closest('.form-step');
                if (step) {
                    const stepNum = parseInt(step.id.split('-')[1]);
                    nextStep(stepNum);

                    // Small delay to allow step transition
                    setTimeout(() => {
                        firstInvalid.focus();
                        firstInvalid.reportValidity();

                        // Scroll to the element if focus didn't do it
                        firstInvalid.scrollIntoView({ behavior: 'smooth', block: 'center' });
                    }, 500);
                }
            }
        } else {
            // Show loading state
            const btn = form.querySelector('button[type="submit"]');
            if (btn) {
                btn.disabled = true;
                btn.innerHTML = '<span>Processing...</span><i class="fas fa-spinner fa-spin ms-2"></i>';
            }
        }
    });

    // --- PROMO CODE AUTO-VALIDATION ---
    const inPromoCode = document.getElementById('in-promo-code');
    const promoLoading = document.getElementById('promo-loading');
    if (inPromoCode) {
        let promoTimeout = null;
        inPromoCode.addEventListener('input', function() {
            clearTimeout(promoTimeout);
            const promoCode = this.value.trim();
            const priceDisplay = document.getElementById('pv-total-final');
            const basePrice = parseInt(priceDisplay.dataset.basePrice);
            
            if (!promoCode) {
                resetPromoCode(basePrice);
                if (promoLoading) promoLoading.style.display = 'none';
                return;
            }
            
            if (promoLoading) promoLoading.style.display = 'block';
            document.getElementById('promo-message').innerHTML = '';
            
            promoTimeout = setTimeout(async function() {
                try {
                    const response = await fetch(`/api/validate-promo?code=${encodeURIComponent(promoCode)}`);
                    const data = await response.json();
                    
                    if (response.ok) {
                        document.getElementById('promo-message').innerHTML = `<span style="color: #10b981;">Promo applied: ${data.discount_percent}% off!</span>`;
                        const discountAmount = (basePrice * data.discount_percent) / 100;
                        const finalAmount = basePrice - discountAmount;
                        priceDisplay.innerHTML = `<span style="text-decoration: line-through; color: #999; font-size: 0.6em; margin-right: 10px; font-weight: normal;">Rp ${basePrice.toLocaleString('id-ID')}</span>Rp ${finalAmount.toLocaleString('id-ID')}`;
                    } else {
                        document.getElementById('promo-message').innerHTML = `<span style="color: #ef4444;">${data.error || 'Invalid promo code'}</span>`;
                        resetPromoCode(basePrice);
                    }
                } catch (err) {
                    document.getElementById('promo-message').innerHTML = `<span style="color: #ef4444;">Connection error</span>`;
                    resetPromoCode(basePrice);
                } finally {
                    if (promoLoading) promoLoading.style.display = 'none';
                }
            }, 800);
        });

        // Check URL parameters for auto-apply
        const urlParams = new URLSearchParams(window.location.search);
        const refCode = urlParams.get('ref') || urlParams.get('promo') || urlParams.get('referral');
        if (refCode) {
            inPromoCode.value = refCode.toUpperCase();
            inPromoCode.dispatchEvent(new Event('input', { bubbles: true }));
        }
    }

    function resetPromoCode(basePrice) {
        document.getElementById('promo-message').innerHTML = '';
        const priceDisplay = document.getElementById('pv-total-final');
        if (priceDisplay) priceDisplay.textContent = `Rp ${basePrice.toLocaleString('id-ID')}`;
    }

    // Lazy load videos Intersection Observer
    document.addEventListener("DOMContentLoaded", function() {
        const lazyVideos = document.querySelectorAll("video.lazy-video");
        if ("IntersectionObserver" in window) {
            const videoObserver = new IntersectionObserver(function(entries, observer) {
                entries.forEach(function(video) {
                    if (video.isIntersecting) {
                        video.target.setAttribute("preload", "auto");
                        video.target.play();
                        videoObserver.unobserve(video.target);
                    } else {
                        video.target.pause();
                    }
                });
            }, { rootMargin: "0px 0px 200px 0px" });

            lazyVideos.forEach(function(video) {
                videoObserver.observe(video);
            });
        }
    });

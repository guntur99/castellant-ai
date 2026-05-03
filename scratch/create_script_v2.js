    let isSlugManuallyEdited = false;
    let currentCategory = 'all';

    function nextStep(step) {
        console.log("Navigating to step:", step);
        if (step === 4) {
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

        document.querySelectorAll('.step-item').forEach(i => i.classList.remove('active'));
        const indicator = document.getElementById('step-' + step + '-indicator');
        if (indicator) {
            indicator.classList.add('active');
        }

        try {
            updatePreview();
        } catch (e) {
            console.warn("updatePreview failed:", e);
        }
        
        window.scrollTo({ top: 150, behavior: 'smooth' });
    }

    async function loadLivePreview() {
        const loader = document.getElementById('preview-loader');
        const iframe = document.getElementById('live-preview-iframe');
        loader.style.display = 'flex';

        const formData = {
            template_name: document.querySelector('input[name="template_name"]:checked').value,
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

    function updateSelectedPlan(name, price) {
        // Update price display in Step 5
        const priceDisplay = document.getElementById('pv-total-final');
        if (priceDisplay) {
            priceDisplay.textContent = `Rp ${price.toLocaleString('id-ID')}`;
            priceDisplay.dataset.basePrice = price;
        }
        
        // Update Plan Name in Review (Step 5)
        const planDisplay = document.querySelector('.plan-selection-name');
        if (planDisplay) {
            planDisplay.textContent = name.charAt(0).toUpperCase() + name.slice(1).toLowerCase() + " Collection";
        }
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
        }
    }

    function prevStep(step) {
        nextStep(step);
    }

    function updatePreview() {
        // Elements to update
        const elName = document.getElementById('pv-name');
        const elSlug = document.getElementById('pv-slug');
        const elPreviewLink = document.getElementById('pv-preview-link');
        const elDate = document.getElementById('pv-date');
        const elVenue = document.getElementById('pv-venue');
        const elPlan = document.getElementById('pv-plan');
        const elTpl = document.getElementById('pv-template');
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
        if (elDate) elDate.innerText = inDate;
        if (elVenue) elVenue.innerText = inAddress ? inVenue + " (" + inAddress + ")" : inVenue;

        // Template logic
        const selectedTplElement = document.querySelector('input[name="template_name"]:checked');
        if (selectedTplElement && elTpl) {
            const val = selectedTplElement.value;
            let displayTpl = "Toktik";
            if (val === 'loveanthem') displayTpl = "Spitapy";
            if (val === 'cinemarry') displayTpl = "Nitflax";
            if (val === 'cairide') displayTpl = "GoJack";
            if (val === 'pinterlove') displayTpl = "Pinteres";
            if (val === 'shopee-live-wedding') displayTpl = "Shoopi";
            if (val === 'tiktok-live-wedding') displayTpl = "Toktik Live";
            if (val === 'we-uber') displayTpl = "Ubar";
            if (val === 'wedding-disney') displayTpl = "Disni";
            if (val === 'wedding-facebook') displayTpl = "Pesbuk";
            if (val === 'wedding-iphone-theme') displayTpl = "iPon";
            if (val === 'wedding-netflix-v2') displayTpl = "Nitflax 2.0";
            if (val === 'wedding-prime') displayTpl = "Primi";
            if (val === 'wedding-wrath-v2') displayTpl = "Wreth";
            if (val === 'wedding-applemusic') displayTpl = "Apel Musik";
            if (val === 'we-capcut') displayTpl = "Kepket";
            if (val === 'bereal-wedding') displayTpl = "BiRil";
            elTpl.innerText = displayTpl;
        }

        // Plan logic
        // Plan logic
        const selectedPlanElement = document.querySelector('input[name="plan_name"]:checked');
        const elPlanSummary = document.querySelector('.plan-selection-name');
        
        let planDisplay = "Basic Collection";
        let priceDisplay = "Rp 50.000";
        let basePrice = "50000";

        if (selectedPlanElement) {
            const planVal = selectedPlanElement.value;
            if (planVal === 'PRO') {
                planDisplay = "Pro Collection";
                priceDisplay = "Rp 100.000";
                basePrice = "100000";
            } else if (planVal === 'ULTIMATE') {
                planDisplay = "Ultimate Collection";
                priceDisplay = "Rp 300.000";
                basePrice = "300000";
            }
        }

        if (elPlanSummary) elPlanSummary.innerText = planDisplay;
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
        const urlParams = new URLSearchParams(window.location.search);

        // Auto-select template
        const tpl = urlParams.get('template');
        if (tpl) {
            const radio = document.querySelector(`input[name="template_name"][value="${tpl}"]`);
            if (radio) radio.checked = true;
        }

        // Auto-select plan if needed (though plan is usually fixed by link)
        // ... any other init ...

        updatePreview();
        toggleReception();
    }

    function openAiAssistant() {
        document.getElementById('aiAssistantModal').style.display = 'flex';
    }

    function closeAiAssistant() {
        const criticalFields = ['in-name', 'in-date', 'in-venue', 'in-bride-name', 'in-groom-name'];
        let missingCount = 0;
        criticalFields.forEach(id => {
            const el = document.getElementById(id);
            if (!el || !el.value.trim()) missingCount++;
        });

        if (missingCount > 0) {
            const msg = `Data penting kamu belum lengkap nih (masih ada ${missingCount} info krusial). \n\nYakin mau stop bantuan AI dan lanjut isi form secara manual?`;
            if (!confirm(msg)) return;
        }
        
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
            'in-bride-full': data.bride_full_name,
            'in-bride-father': data.bride_father,
            'in-bride-mother': data.bride_mother,
            'in-groom-name': data.groom_name,
            'in-groom-full': data.groom_full_name,
            'in-groom-father': data.groom_father,
            'in-groom-mother': data.groom_mother,
            'in-reception-date': data.reception_date,
            'in-reception-time': data.reception_time,
            'in-reception-venue': data.reception_venue,
            'in-reception-address': data.reception_address,
            'in-reception-maps': data.reception_maps,
            'quote-textarea': data.quote_text,
            'quote-source': data.quote_source
        };

        for (const [id, value] of Object.entries(fieldMap)) {
            const el = document.getElementById(id);
            if (el && value && value !== "") {
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

        const btn = document.getElementById('btnParseForm');
        btn.disabled = true;
        btn.innerHTML = '<span>⏳ AI sedang berpikir...</span>';

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
            const result = JSON.parse(rawResult.text);
            
            // Update session ID if it was newly created
            if (rawResult.session_id) {
                currentAiSessionId = rawResult.session_id;
                localStorage.setItem('ai_session_id', currentAiSessionId);
            }

            const parsed = JSON.parse(rawResult.text);
            const data = parsed.data;

            fillFormFromData(data);
            appendMessageToChat('assistant', parsed.reply || "Data sudah saya proses!");

            if (parsed.missing && parsed.missing.length === 0) {
                showAiToast("🎉 Semua data penting sudah lengkap!");
            }
        } catch (err) {
            console.error(err);
            showAiToast("❌ Gagal memproses data.");
        } finally {
            btn.disabled = false;
            btn.innerHTML = '<span>🚀 Kirim Pesan</span>';
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

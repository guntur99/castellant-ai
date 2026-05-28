-- Seed Keraton Dark Invitation template with status 'PUBLISHED'
INSERT INTO templates (id, slug, title, description, category, preview_img, status, is_featured)
VALUES (
    'keraton-dark-invitation',
    'keraton-dark-invitation',
    'Keraton Dark',
    'Keagungan adat Jawa dalam balutan kemewahan gelap. Desain premium dengan nuansa keraton yang megah, elegan, dan penuh tradisi.',
    'royal',
    '/static/img/keraton-dark-preview.png',
    'PUBLISHED',
    true
)
ON CONFLICT (id) DO UPDATE SET
    status = 'PUBLISHED',
    is_featured = true,
    category = 'royal';

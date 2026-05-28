-- Update template status for super-wedbros to DRAFT
UPDATE templates SET status = 'DRAFT' WHERE slug = 'super-wedbros' OR id = 'super-wedbros';

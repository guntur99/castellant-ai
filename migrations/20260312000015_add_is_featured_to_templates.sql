-- Add is_featured column to templates table
ALTER TABLE templates ADD COLUMN IF NOT EXISTS is_featured BOOLEAN NOT NULL DEFAULT FALSE;

-- Set some default featured templates for the home page
UPDATE templates SET is_featured = TRUE WHERE id IN ('trendvibe', 'loveanthem', 'cinemarry', 'cairide', 'pinterlove', 'wedding-disney');

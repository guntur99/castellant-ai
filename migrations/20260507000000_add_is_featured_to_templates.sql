-- Add is_featured column to templates table
ALTER TABLE templates ADD COLUMN IF NOT EXISTS is_featured BOOLEAN NOT NULL DEFAULT FALSE;

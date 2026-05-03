-- Add plan_type column to invitations table
ALTER TABLE invitations ADD COLUMN IF NOT EXISTS plan_type TEXT DEFAULT 'BASIC';

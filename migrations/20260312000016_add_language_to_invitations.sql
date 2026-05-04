-- Add language column to invitations
ALTER TABLE invitations ADD COLUMN IF NOT EXISTS language VARCHAR(10) DEFAULT 'id';

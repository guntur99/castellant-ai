-- Add initial payment fields
ALTER TABLE invitations ADD COLUMN IF NOT EXISTS payment_proof TEXT;
ALTER TABLE invitations ADD COLUMN IF NOT EXISTS payment_status TEXT DEFAULT 'pending';

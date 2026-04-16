-- Add Mayar specific payment fields and plan name
ALTER TABLE invitations ADD COLUMN IF NOT EXISTS payment_link TEXT;
ALTER TABLE invitations ADD COLUMN IF NOT EXISTS payment_invoice_id TEXT;
ALTER TABLE invitations ADD COLUMN IF NOT EXISTS plan_name TEXT;

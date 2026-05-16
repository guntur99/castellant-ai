-- Add deleted_at column to invitations table
ALTER TABLE invitations ADD COLUMN deleted_at TIMESTAMPTZ;

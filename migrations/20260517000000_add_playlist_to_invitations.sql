ALTER TABLE invitations ADD COLUMN playlist JSONB DEFAULT '[]'::jsonb;

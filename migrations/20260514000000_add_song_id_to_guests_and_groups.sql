-- Add song_id to invitation_groups
ALTER TABLE invitation_groups ADD COLUMN IF NOT EXISTS song_id UUID REFERENCES songs(id);

-- Add song_id to guests
ALTER TABLE guests ADD COLUMN IF NOT EXISTS song_id UUID REFERENCES songs(id);

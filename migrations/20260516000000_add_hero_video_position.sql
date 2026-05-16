-- Add hero_video_position column to invitations table
ALTER TABLE invitations ADD COLUMN IF NOT EXISTS hero_video_position INTEGER DEFAULT 50;

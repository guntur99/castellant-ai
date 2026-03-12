-- Seed or Update the song with the local file path
INSERT INTO songs (title, artist, file_path, is_active)
VALUES ('Lover', 'Taylor Swift', '/static/music/lover.mp3', true)
ON CONFLICT DO NOTHING;

UPDATE songs 
SET file_path = '/static/music/lover.mp3', is_active = true 
WHERE title = 'Lover';

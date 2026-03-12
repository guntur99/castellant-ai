-- Restore migration 20260312000001
UPDATE songs 
SET file_path = '/static/music/lover.mp3', is_active = true 
WHERE title = 'Lover';

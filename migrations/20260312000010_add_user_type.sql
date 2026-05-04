ALTER TABLE users ADD COLUMN IF NOT EXISTS role VARCHAR(20) DEFAULT 'USER';

-- Tetapkan superadmin awal berdasarkan email
UPDATE users SET role = 'SUPERADMIN' 
WHERE email IN ('gugunguntur99@gmail.com') 
OR email = (SELECT value FROM (SELECT current_setting('app.admin_email', true) as value) s WHERE value IS NOT NULL);

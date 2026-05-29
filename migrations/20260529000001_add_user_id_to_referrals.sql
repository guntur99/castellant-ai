ALTER TABLE referrals ADD COLUMN user_id UUID REFERENCES users(id) ON DELETE CASCADE;
ALTER TABLE referrals RENAME COLUMN commission_amount TO commission_percent;
ALTER TABLE referrals ALTER COLUMN commission_percent SET DEFAULT 10;

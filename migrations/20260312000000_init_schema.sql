-- Users Table for Google Auth and general info
CREATE TABLE IF NOT EXISTS users (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    google_id TEXT UNIQUE,
    email TEXT UNIQUE NOT NULL,
    name TEXT,
    avatar_url TEXT,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP
);

-- Songs Table
CREATE TABLE IF NOT EXISTS songs (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    title TEXT NOT NULL,
    artist TEXT NOT NULL,
    file_path TEXT NOT NULL,
    audio_data BYTEA,
    is_active BOOLEAN DEFAULT false
);


-- Invitations (Orders) Table
CREATE TABLE IF NOT EXISTS invitations (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID REFERENCES users(id),
    slug TEXT UNIQUE NOT NULL,
    couple_name_short TEXT NOT NULL,
    template_name TEXT DEFAULT 'vintage',
    event_date TEXT NOT NULL,
    song_id UUID REFERENCES songs(id),
    
    -- Bride & Groom info
    bride_data JSONB NOT NULL,
    groom_data JSONB NOT NULL,
    
    -- Event details
    ceremony_data JSONB NOT NULL,
    reception_data JSONB NOT NULL,
    quote_data JSONB NOT NULL,
    
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP
);

-- Photos Table
CREATE TABLE IF NOT EXISTS invitation_photos (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    invitation_id UUID REFERENCES invitations(id) ON DELETE CASCADE,
    url TEXT NOT NULL,
    photo_type TEXT NOT NULL,
    "order" INTEGER DEFAULT 0
);

-- Gift Accounts Table
CREATE TABLE IF NOT EXISTS gift_accounts (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    invitation_id UUID REFERENCES invitations(id) ON DELETE CASCADE,
    bank_name TEXT NOT NULL,
    account_number TEXT NOT NULL,
    account_holder TEXT NOT NULL
);

-- RSVP Table
CREATE TABLE IF NOT EXISTS rsvps (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    invitation_id UUID REFERENCES invitations(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    attendance TEXT NOT NULL,
    guests INTEGER DEFAULT 1,
    message TEXT,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP
);

-- Seed Initial Song
INSERT INTO songs (title, artist, file_path, is_active)
VALUES ('Lover', 'Taylor Swift', 'https://www.soundhelix.com/examples/mp3/SoundHelix-Song-1.mp3', true);

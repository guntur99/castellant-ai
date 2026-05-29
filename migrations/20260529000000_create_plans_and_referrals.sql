-- Create plans table
CREATE TABLE plans (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    code VARCHAR(50) NOT NULL UNIQUE,
    name VARCHAR(100) NOT NULL,
    price INTEGER NOT NULL,
    template_limit INTEGER NOT NULL,
    features JSONB,
    is_active BOOLEAN DEFAULT true,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- Insert initial seeds
INSERT INTO plans (code, name, price, template_limit, features) VALUES
('NOBLE', 'Noble', 50000, 3, '["Akses ke 3 Tema Premium", "Masa Aktif 6 Bulan", "Fitur RSVP Dasar"]'::jsonb),
('ROYAL', 'Royal', 100000, 7, '["Akses ke 7 Tema Premium", "Masa Aktif 1 Tahun", "Fitur RSVP Lanjutan", "Custom Song"]'::jsonb),
('DYNASTY', 'Dynasty', 300000, 999, '["Semua Tema Premium", "Masa Aktif Selamanya", "RSVP VIP & AI Chat", "Prioritas Support"]'::jsonb);

-- Create referrals table
CREATE TABLE referrals (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    code VARCHAR(50) NOT NULL UNIQUE,
    referrer_name VARCHAR(100) NOT NULL,
    discount_percent INTEGER NOT NULL DEFAULT 0,
    commission_amount INTEGER NOT NULL DEFAULT 0,
    usage_count INTEGER NOT NULL DEFAULT 0,
    is_active BOOLEAN DEFAULT true,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

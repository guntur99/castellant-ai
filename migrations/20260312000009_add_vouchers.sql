CREATE TABLE IF NOT EXISTS vouchers (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    code VARCHAR(50) UNIQUE NOT NULL,
    discount_percent INT NOT NULL, -- 1-100
    valid_until TIMESTAMPTZ,
    usage_limit INT,
    usage_count INT DEFAULT 0,
    is_active BOOLEAN DEFAULT TRUE,
    created_at TIMESTAMPTZ DEFAULT NOW()
);

-- Tambahkan field voucher ke tabel bookings
ALTER TABLE bookings ADD COLUMN IF NOT EXISTS voucher_code VARCHAR(50);
ALTER TABLE bookings ADD COLUMN IF NOT EXISTS discount_amount INT DEFAULT 0;

-- Seed some initial vouchers
INSERT INTO vouchers (code, discount_percent, usage_limit) VALUES ('CASTELLANTAI', 20, 100) ON CONFLICT DO NOTHING;
INSERT INTO vouchers (code, discount_percent, usage_limit) VALUES ('PROMOBAHAGIA', 50, 10) ON CONFLICT DO NOTHING;

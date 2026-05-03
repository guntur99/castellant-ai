-- Create Guests Table
CREATE TABLE IF NOT EXISTS guests (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    invitation_id UUID REFERENCES invitations(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    category TEXT,
    template_override TEXT,
    slug TEXT NOT NULL,
    is_sent BOOLEAN DEFAULT false,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP
);

-- Ensure guest slug is unique within an invitation
CREATE UNIQUE INDEX IF NOT EXISTS idx_guest_slug_invitation ON guests(invitation_id, slug);

-- Create Invitation Groups Table for template mapping
CREATE TABLE IF NOT EXISTS invitation_groups (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    invitation_id UUID REFERENCES invitations(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    template_name TEXT NOT NULL,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP
);

-- Ensure group name is unique within an invitation
CREATE UNIQUE INDEX IF NOT EXISTS idx_group_name_invitation ON invitation_groups(invitation_id, name);

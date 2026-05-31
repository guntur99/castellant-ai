CREATE TABLE blog_posts (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    slug VARCHAR(255) NOT NULL UNIQUE,
    title VARCHAR(255) NOT NULL,
    content_html TEXT NOT NULL,
    meta_description TEXT,
    meta_keywords TEXT,
    image_url VARCHAR(255),
    is_published BOOLEAN DEFAULT false,
    published_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Trigger for updated_at
CREATE OR REPLACE FUNCTION update_modified_column_blog()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ language 'plpgsql';

CREATE TRIGGER update_blog_posts_modtime
    BEFORE UPDATE ON blog_posts
    FOR EACH ROW
    EXECUTE FUNCTION update_modified_column_blog();

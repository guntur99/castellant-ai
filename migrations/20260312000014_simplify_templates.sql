-- Remove price and plan columns as they are no longer needed
ALTER TABLE templates DROP COLUMN IF EXISTS price;
ALTER TABLE templates DROP COLUMN IF EXISTS plan;

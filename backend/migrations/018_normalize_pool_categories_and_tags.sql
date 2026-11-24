-- Migration: normalise pool categories and tags
--
-- Categories and tags are now stored canonically (trimmed, lower-case,
-- tags de-duplicated) so filtering, interest matching and tag counts do not
-- fragment across "Crypto" / "crypto" / " crypto ". New writes are normalised
-- by the application (`tags::normalize_category` / `tags::normalize_tags`);
-- this brings existing rows in line.

UPDATE pools
SET category = lower(btrim(category))
WHERE category <> lower(btrim(category));

UPDATE pools
SET tags = ARRAY(
    SELECT DISTINCT lower(btrim(t))
    FROM unnest(tags) AS t
    WHERE btrim(t) <> ''
    ORDER BY 1
)
WHERE tags <> '{}';

-- Category counts power the `GET /api/v1/categories` filter dropdown.
CREATE INDEX IF NOT EXISTS idx_pools_state_lower_category
    ON pools (state, lower(category));

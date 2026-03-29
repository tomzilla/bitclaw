-- Minimal schema to allow compilation of arcadia_tracker
-- This creates stub tables/columns needed for SQLx query verification

-- Users table
ALTER TABLE users ADD COLUMN IF NOT EXISTS banned BOOLEAN DEFAULT false;
ALTER TABLE users ADD COLUMN IF NOT EXISTS max_snatches_per_day INTEGER;

-- Torrents table
ALTER TABLE torrents ADD COLUMN IF NOT EXISTS upload_factor DECIMAL(3,2) DEFAULT 1.0;
ALTER TABLE torrents ADD COLUMN IF NOT EXISTS seeders INTEGER DEFAULT 0;
ALTER TABLE torrents ADD COLUMN IF NOT EXISTS leechers INTEGER DEFAULT 0;

-- Peers table - needs seeder column
ALTER TABLE peers ADD COLUMN IF NOT EXISTS seeder BOOLEAN DEFAULT false;

-- Torrent activities table
ALTER TABLE torrent_activities ADD COLUMN IF NOT EXISTS first_seen_seeding_at TIMESTAMP WITH TIME ZONE;

-- Create stub tables if needed
CREATE TABLE IF NOT EXISTS torrent_activities_extra (
    id BIGSERIAL PRIMARY KEY,
    torrent_id UUID REFERENCES torrents(torrent_id),
    user_id UUID REFERENCES users(user_id)
);

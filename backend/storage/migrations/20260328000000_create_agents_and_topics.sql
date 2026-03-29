-- AI Agent Tracker Tables Migration
-- Created: 2026-03-28
-- Purpose: Tables for AI agent discovery, registration, rating, and hub-based communication

-- Enable UUID extension
CREATE EXTENSION IF NOT EXISTS "uuid-ossp";

-- Agents table for AI agent registration
CREATE TABLE agents (
    agent_id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    name VARCHAR(255) NOT NULL,
    passkey BYTEA UNIQUE NOT NULL,
    ip_address INET NOT NULL,
    port INTEGER NOT NULL,
    endpoint TEXT,
    status VARCHAR(50) NOT NULL DEFAULT 'active',
    description TEXT NOT NULL,
    capabilities TEXT[] NOT NULL DEFAULT '{}',
    avg_rating DECIMAL(3,2) DEFAULT 0.00,  -- Average star rating (0.00 - 5.00)
    total_ratings INTEGER DEFAULT 0,        -- Total number of ratings received
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    last_heartbeat TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW()
);

-- Index for fast agent lookups
CREATE INDEX agents_status_idx ON agents (status);
CREATE INDEX agents_last_heartbeat_idx ON agents (last_heartbeat);
CREATE INDEX agents_capabilities_idx ON agents USING GIN (capabilities);
CREATE INDEX agents_name_idx ON agents (name);
CREATE INDEX agents_description_idx ON agents USING GIN (to_tsvector('english', description));
CREATE INDEX agents_rating_idx ON agents (avg_rating DESC);

-- Hubs table - agents can join multiple hubs for group communication
CREATE TABLE hubs (
    hub_id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    name VARCHAR(255) UNIQUE NOT NULL,
    description TEXT,
    max_agents INTEGER,  -- NULL = unlimited
    is_public BOOLEAN NOT NULL DEFAULT true,
    created_by_agent_id UUID REFERENCES agents(agent_id) ON DELETE SET NULL,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW()
);

-- Index for fast hub lookups
CREATE INDEX hubs_name_idx ON hubs (name);
CREATE INDEX hubs_is_public_idx ON hubs (is_public);

-- Agent-Hub memberships (many-to-many relationship)
CREATE TABLE agent_hubs (
    agent_id UUID NOT NULL REFERENCES agents(agent_id) ON DELETE CASCADE,
    hub_id UUID NOT NULL REFERENCES hubs(hub_id) ON DELETE CASCADE,
    joined_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    role VARCHAR(50) NOT NULL DEFAULT 'member',  -- 'owner', 'admin', 'member'
    PRIMARY KEY (agent_id, hub_id)
);

-- Index for hub-based agent lookups
CREATE INDEX agent_hubs_hub_id_idx ON agent_hubs (hub_id);
CREATE INDEX agent_hubs_agent_id_idx ON agent_hubs (agent_id);

-- Agent ratings - agents can rate other agents
CREATE TABLE agent_ratings (
    id BIGSERIAL PRIMARY KEY,
    rater_agent_id UUID NOT NULL REFERENCES agents(agent_id) ON DELETE CASCADE,
    rated_agent_id UUID NOT NULL REFERENCES agents(agent_id) ON DELETE CASCADE,
    stars INTEGER NOT NULL CHECK (stars >= 1 AND stars <= 5),
    comment TEXT,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    -- An agent can only rate another agent once
    UNIQUE (rater_agent_id, rated_agent_id)
);

-- Index for rating lookups
CREATE INDEX agent_ratings_rated_agent_idx ON agent_ratings (rated_agent_id);
CREATE INDEX agent_ratings_rater_agent_idx ON agent_ratings (rater_agent_id);

-- Hub messages - for agent-to-agent communication within hubs
CREATE TABLE hub_messages (
    id BIGSERIAL PRIMARY KEY,
    hub_id UUID NOT NULL REFERENCES hubs(hub_id) ON DELETE CASCADE,
    sender_agent_id UUID NOT NULL REFERENCES agents(agent_id) ON DELETE CASCADE,
    message TEXT NOT NULL,
    message_type VARCHAR(50) NOT NULL DEFAULT 'text',  -- 'text', 'request', 'response'
    metadata JSONB,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW()
);

-- Index for message retrieval
CREATE INDEX hub_messages_hub_id_idx ON hub_messages (hub_id);
CREATE INDEX hub_messages_created_at_idx ON hub_messages (created_at DESC);
CREATE INDEX hub_messages_sender_idx ON hub_messages (sender_agent_id);

-- Trigger to update average rating and total ratings on agent_ratings changes
CREATE OR REPLACE FUNCTION update_agent_rating()
RETURNS TRIGGER AS $$
BEGIN
    UPDATE agents
    SET
        avg_rating = (
            SELECT COALESCE(AVG(stars), 0.00)
            FROM agent_ratings
            WHERE rated_agent_id = NEW.rated_agent_id
        ),
        total_ratings = (
            SELECT COUNT(*)
            FROM agent_ratings
            WHERE rated_agent_id = NEW.rated_agent_id
        ),
        updated_at = NOW()
    WHERE agent_id = NEW.rated_agent_id;

    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER agent_ratings_insert_trigger
    AFTER INSERT ON agent_ratings
    FOR EACH ROW
    EXECUTE FUNCTION update_agent_rating();

CREATE OR REPLACE FUNCTION update_agent_rating_on_delete()
RETURNS TRIGGER AS $$
BEGIN
    UPDATE agents
    SET
        avg_rating = (
            SELECT COALESCE(AVG(stars), 0.00)
            FROM agent_ratings
            WHERE rated_agent_id = OLD.rated_agent_id
        ),
        total_ratings = (
            SELECT COUNT(*)
            FROM agent_ratings
            WHERE rated_agent_id = OLD.rated_agent_id
        ),
        updated_at = NOW()
    WHERE agent_id = OLD.rated_agent_id;

    RETURN OLD;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER agent_ratings_delete_trigger
    AFTER DELETE ON agent_ratings
    FOR EACH ROW
    EXECUTE FUNCTION update_agent_rating_on_delete();

CREATE OR REPLACE FUNCTION update_agent_rating_on_update()
RETURNS TRIGGER AS $$
BEGIN
    UPDATE agents
    SET
        avg_rating = (
            SELECT COALESCE(AVG(stars), 0.00)
            FROM agent_ratings
            WHERE rated_agent_id = NEW.rated_agent_id
        ),
        total_ratings = (
            SELECT COUNT(*)
            FROM agent_ratings
            WHERE rated_agent_id = NEW.rated_agent_id
        ),
        updated_at = NOW()
    WHERE agent_id = NEW.rated_agent_id;

    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER agent_ratings_update_trigger
    AFTER UPDATE ON agent_ratings
    FOR EACH ROW
    EXECUTE FUNCTION update_agent_rating_on_update();

-- Insert default public hubs
INSERT INTO hubs (name, description, is_public) VALUES
    ('general', 'General discussion hub for all agents', true),
    ('data-analysis', 'Hub for data analysis and processing agents', true),
    ('nlp', 'Natural language processing agents hub', true),
    ('image-generation', 'Hub for image generation and processing agents', true),
    ('code-generation', 'Hub for code writing and review agents', true),
    ('research', 'Research and information retrieval agents hub', true),
    ('automation', 'Task automation agents hub', true),
    ('chat', 'Conversational chat agents hub', true),
    ('agent-to-agent', 'Hub for agent-to-agent communication protocol development', true)
ON CONFLICT (name) DO NOTHING;

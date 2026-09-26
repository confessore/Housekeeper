CREATE TABLE IF NOT EXISTS users (
    id BIGSERIAL PRIMARY KEY,
    discord_id TEXT NOT NULL UNIQUE,
    username TEXT NOT NULL,
    steam_id BIGINT,
    steam_persona_name TEXT,
    friend_code TEXT,
    rank_tier INTEGER,
    wins INTEGER NOT NULL DEFAULT 0,
    losses INTEGER NOT NULL DEFAULT 0,
    rank_synced_at TIMESTAMPTZ,
    inhouse_banned BOOLEAN NOT NULL DEFAULT FALSE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS guilds (
    id BIGSERIAL PRIMARY KEY,
    discord_guild_id BIGINT NOT NULL UNIQUE,
    name TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS role_tiers (
    id BIGSERIAL PRIMARY KEY,
    guild_id BIGINT NOT NULL REFERENCES guilds(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    is_moderator BOOLEAN NOT NULL DEFAULT FALSE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(guild_id, name)
);

CREATE TABLE IF NOT EXISTS role_mappings (
    guild_id BIGINT NOT NULL REFERENCES guilds(id) ON DELETE CASCADE,
    discord_role_id TEXT NOT NULL,
    role_tier_id BIGINT REFERENCES role_tiers(id) ON DELETE CASCADE,
    PRIMARY KEY(guild_id, discord_role_id)
);

CREATE INDEX IF NOT EXISTS role_mappings_tier_idx ON role_mappings(role_tier_id);

CREATE TABLE IF NOT EXISTS discord_roles_cache (
    discord_role_id TEXT PRIMARY KEY,
    guild_id BIGINT NOT NULL,
    name TEXT NOT NULL,
    position BIGINT NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS member_roles_cache (
    user_discord_id TEXT NOT NULL,
    discord_role_id TEXT NOT NULL,
    PRIMARY KEY(user_discord_id, discord_role_id)
);

CREATE TABLE IF NOT EXISTS infractions (
    id BIGSERIAL PRIMARY KEY,
    target_discord_id TEXT NOT NULL,
    guild_id BIGINT NOT NULL,
    kind TEXT NOT NULL CHECK(kind IN ('warn','ban','unban')),
    reason TEXT NOT NULL,
    issued_by_discord_id TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS infractions_target_idx
    ON infractions(guild_id, target_discord_id, created_at DESC);

CREATE TABLE IF NOT EXISTS lobbies (
    id BIGSERIAL PRIMARY KEY,
    guild_id BIGINT NOT NULL REFERENCES guilds(id) ON DELETE CASCADE,
    number INTEGER NOT NULL CHECK(number > 0),
    created_by_discord_id TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    expires_at TIMESTAMPTZ NOT NULL,
    UNIQUE(guild_id, number)
);

CREATE INDEX IF NOT EXISTS lobbies_active_idx ON lobbies(guild_id, expires_at);

CREATE TABLE IF NOT EXISTS lobby_members (
    lobby_id BIGINT NOT NULL REFERENCES lobbies(id) ON DELETE CASCADE,
    discord_id TEXT NOT NULL,
    joined_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    added_by_discord_id TEXT NOT NULL,
    rank_tier INTEGER,
    wins INTEGER NOT NULL DEFAULT 0,
    losses INTEGER NOT NULL DEFAULT 0,
    PRIMARY KEY(lobby_id, discord_id)
);

CREATE INDEX IF NOT EXISTS lobby_members_joined_idx
    ON lobby_members(lobby_id, joined_at);

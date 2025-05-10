-- Create users table
CREATE TABLE users (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    username VARCHAR(255) NOT NULL UNIQUE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    last_login TIMESTAMPTZ,
    total_games_played INTEGER NOT NULL DEFAULT 0
);

-- Create game_sessions table
CREATE TABLE game_sessions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    lobby_id VARCHAR(255) NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    ended_at TIMESTAMPTZ,
    player_count INTEGER NOT NULL DEFAULT 1,
    settings JSONB NOT NULL DEFAULT '{}'::jsonb
);

-- Create game_actions table
CREATE TABLE game_actions (
    id BIGSERIAL PRIMARY KEY,
    game_id UUID NOT NULL REFERENCES game_sessions(id),
    user_id UUID REFERENCES users(id),
    action_type VARCHAR(255) NOT NULL,
    action_data JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Create indexes
CREATE INDEX idx_users_username ON users(username);
CREATE INDEX idx_game_sessions_lobby_id ON game_sessions(lobby_id);
CREATE INDEX idx_game_sessions_created_at ON game_sessions(created_at);
CREATE INDEX idx_game_actions_game_id ON game_actions(game_id);
CREATE INDEX idx_game_actions_user_id ON game_actions(user_id);
CREATE INDEX idx_game_actions_created_at ON game_actions(created_at);

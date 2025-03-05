-- Create game_actions table
CREATE TABLE game_actions (
    id BIGSERIAL PRIMARY KEY,
    game_id UUID NOT NULL REFERENCES game_sessions(id),
    user_id UUID REFERENCES users(id),
    action_type TEXT NOT NULL,
    action_data JSONB NOT NULL DEFAULT '{}'::JSONB,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Add indexes for faster lookups
CREATE INDEX idx_game_actions_game_id ON game_actions(game_id);
CREATE INDEX idx_game_actions_user_id ON game_actions(user_id);
CREATE INDEX idx_game_actions_type ON game_actions(action_type);

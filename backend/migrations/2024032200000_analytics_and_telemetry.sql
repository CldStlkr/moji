-- Add personal learning analytics and heartbeat telemetry to the specific users
ALTER TABLE users ADD COLUMN last_seen_at TIMESTAMPTZ DEFAULT NOW();
ALTER TABLE users ADD COLUMN games_won INTEGER NOT NULL DEFAULT 0;
ALTER TABLE users ADD COLUMN words_guessed_correctly INTEGER NOT NULL DEFAULT 0;
ALTER TABLE users ADD COLUMN total_score BIGINT NOT NULL DEFAULT 0;

-- Create the singleton cache table for aggregate platform health metrics
CREATE TABLE global_stats (
    id INT PRIMARY KEY DEFAULT 1 CHECK (id = 1),
    total_unique_visitors BIGINT NOT NULL DEFAULT 0,
    total_games_played BIGINT NOT NULL DEFAULT 0,
    total_words_guessed BIGINT NOT NULL DEFAULT 0,
    peak_concurrent_players INTEGER NOT NULL DEFAULT 0,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Seed the initial tracking row since id maps strictly to 1
INSERT INTO global_stats (id) VALUES (1);

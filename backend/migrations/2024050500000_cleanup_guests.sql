-- Update game_actions foreign key to allow user deletion
ALTER TABLE game_actions 
DROP CONSTRAINT IF EXISTS game_actions_user_id_fkey,
ADD CONSTRAINT game_actions_user_id_fkey 
    FOREIGN KEY (user_id) 
    REFERENCES users(id) 
    ON DELETE SET NULL;

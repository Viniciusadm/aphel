ALTER TABLE goals DROP FOREIGN KEY goals_user_fk;
ALTER TABLE goals DROP FOREIGN KEY goals_team_fk;
ALTER TABLE goals
  ADD CONSTRAINT goals_user_fk FOREIGN KEY (user_id) REFERENCES users (id) ON DELETE SET NULL,
  ADD CONSTRAINT goals_team_fk FOREIGN KEY (team_id) REFERENCES teams (id) ON DELETE SET NULL;

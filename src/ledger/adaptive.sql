CREATE TABLE mode_history(run_id TEXT NOT NULL REFERENCES runs(id),revision INTEGER NOT NULL,previous_graph TEXT NOT NULL,policy_digest TEXT NOT NULL,PRIMARY KEY(run_id,revision));

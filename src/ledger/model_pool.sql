CREATE TABLE model_pools(run_id TEXT PRIMARY KEY REFERENCES runs(id),revision INTEGER NOT NULL,document TEXT NOT NULL);

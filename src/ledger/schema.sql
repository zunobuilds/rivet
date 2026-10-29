CREATE TABLE meta(schema_version INTEGER NOT NULL, project_uuid TEXT NOT NULL, next_event_sequence INTEGER NOT NULL, owner_epoch INTEGER NOT NULL);
CREATE TABLE sessions(id TEXT PRIMARY KEY, created_at INTEGER NOT NULL, config_version INTEGER NOT NULL);
CREATE TABLE runs(id TEXT PRIMARY KEY, session_id TEXT NOT NULL REFERENCES sessions(id), mode TEXT NOT NULL, state TEXT NOT NULL, owner_epoch INTEGER NOT NULL, revision INTEGER NOT NULL);
CREATE TABLE agents(id TEXT PRIMARY KEY, run_id TEXT NOT NULL REFERENCES runs(id), role TEXT NOT NULL);
CREATE TABLE tasks(id TEXT PRIMARY KEY, run_id TEXT NOT NULL REFERENCES runs(id), owner_agent_id TEXT NOT NULL REFERENCES agents(id), state TEXT NOT NULL, revision INTEGER NOT NULL);
CREATE TABLE operations(id TEXT PRIMARY KEY, run_id TEXT NOT NULL REFERENCES runs(id), task_id TEXT NOT NULL REFERENCES tasks(id), actor_id TEXT NOT NULL REFERENCES agents(id), descriptor_digest TEXT NOT NULL, proposal_digest TEXT NOT NULL, state TEXT NOT NULL, revision INTEGER NOT NULL);
CREATE TABLE grants(id TEXT PRIMARY KEY, operation_id TEXT NOT NULL REFERENCES operations(id), epoch INTEGER NOT NULL, expiry INTEGER NOT NULL, scope TEXT NOT NULL);
CREATE TABLE events(sequence INTEGER PRIMARY KEY, entity_id TEXT NOT NULL, actor TEXT NOT NULL, kind TEXT NOT NULL, payload TEXT NOT NULL);

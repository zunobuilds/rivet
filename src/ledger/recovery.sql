CREATE TABLE operation_intents(operation_id TEXT PRIMARY KEY REFERENCES operations(id),class TEXT NOT NULL,metadata TEXT NOT NULL);
CREATE TABLE owned_workers(operation_id TEXT PRIMARY KEY REFERENCES operations(id),unit TEXT NOT NULL UNIQUE,description TEXT NOT NULL,owner_epoch INTEGER NOT NULL);
CREATE TABLE artifacts(digest TEXT PRIMARY KEY,size INTEGER NOT NULL CHECK(size>=0 AND size<=67108864),media_type TEXT NOT NULL);
CREATE TABLE operation_artifacts(operation_id TEXT NOT NULL REFERENCES operations(id),digest TEXT NOT NULL REFERENCES artifacts(digest),PRIMARY KEY(operation_id,digest));
CREATE TABLE recovery_control(id INTEGER PRIMARY KEY CHECK(id=1),required INTEGER NOT NULL CHECK(required IN (0,1)));
INSERT INTO recovery_control VALUES(1,0);

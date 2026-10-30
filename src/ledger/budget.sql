CREATE TABLE run_budgets(
 run_id TEXT PRIMARY KEY REFERENCES runs(id), token_limit INTEGER NOT NULL CHECK(token_limit>0),
 token_charged INTEGER NOT NULL DEFAULT 0 CHECK(token_charged>=0), token_reserved INTEGER NOT NULL DEFAULT 0 CHECK(token_reserved>=0),
 request_limit INTEGER NOT NULL CHECK(request_limit>0), requests INTEGER NOT NULL DEFAULT 0 CHECK(requests>=0),
 cost_limit INTEGER NOT NULL CHECK(cost_limit>=0), cost_charged INTEGER NOT NULL DEFAULT 0 CHECK(cost_charged>=0), cost_reserved INTEGER NOT NULL DEFAULT 0 CHECK(cost_reserved>=0)
);
CREATE TABLE inference_reservations(
 request_id TEXT PRIMARY KEY, run_id TEXT NOT NULL REFERENCES run_budgets(run_id),
 tokens INTEGER NOT NULL CHECK(tokens>0), cost INTEGER NOT NULL CHECK(cost>=0), request_digest TEXT NOT NULL,
 permission_epoch INTEGER NOT NULL, state TEXT NOT NULL CHECK(state IN ('reserved','settled','unknown')), input_tokens INTEGER, output_tokens INTEGER
);

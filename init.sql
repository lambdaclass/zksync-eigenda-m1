CREATE TABLE IF NOT EXISTS blob_proofs (
    id SERIAL PRIMARY KEY,
    blob_id TEXT UNIQUE,
    proof TEXT,
    failed BOOLEAN DEFAULT FALSE
);

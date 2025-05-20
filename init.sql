CREATE TABLE IF NOT EXISTS blob_proofs (
    id SERIAL PRIMARY KEY,
    blob_id TEXT,
    proof TEXT
);

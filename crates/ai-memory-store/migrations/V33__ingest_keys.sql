-- Idempotency keys for hook ingest. A spooled event carries a stable
-- client-generated key; the writer records it in the same transaction as the
-- observation/handoff it produced, so a batch retried after a lost response
-- is recognized and skipped instead of duplicating rows. Swept by TTL from
-- the maintenance scheduler (keys only need to outlive the client spool).
CREATE TABLE ingest_keys (
    key     TEXT PRIMARY KEY,
    seen_at INTEGER NOT NULL
) WITHOUT ROWID;

CREATE INDEX idx_ingest_keys_seen_at ON ingest_keys (seen_at);

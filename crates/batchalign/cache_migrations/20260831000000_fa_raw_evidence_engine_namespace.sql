-- Repair the v0.3.0 forced-alignment raw-evidence namespace defect.
--
-- Schema-2 payloads own the exact selected-worker version, so their SQLite
-- label can be restored without inference or guesswork. Schema-1 payloads own
-- only the requested engine family. If that family contradicts the stored
-- model-version family, no exact producer version survives and the only
-- truthful migration is quarantine followed by removal from live lookup.
-- Correctly labelled schema-1 evidence stays replayable through its explicit
-- legacy-cache-namespace admission path.

CREATE TABLE IF NOT EXISTS cache_quarantine (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    key TEXT NOT NULL,
    task TEXT NOT NULL,
    engine_version TEXT NOT NULL,
    batchalign_version TEXT NOT NULL,
    created_at TEXT NOT NULL,
    data BLOB NOT NULL,
    quarantined_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    reason TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_cache_quarantine_reason
    ON cache_quarantine(reason);

INSERT INTO cache_quarantine (
    key,
    task,
    engine_version,
    batchalign_version,
    created_at,
    data,
    reason
)
SELECT
    key,
    task,
    engine_version,
    batchalign_version,
    created_at,
    data,
    'legacy_fa_raw_evidence_engine_namespace_unprovable'
FROM cache_entries
WHERE task = 'forced_alignment_raw_evidence'
  AND json_valid(data)
  AND json_extract(data, '$.schema_version') = 1
  AND (
    (
      json_extract(data, '$.requested_engine') = 'whisper_fa'
      AND engine_version NOT GLOB 'whisper-fa-*'
    )
    OR (
      json_extract(data, '$.requested_engine') = 'wav2vec_fa'
      AND engine_version NOT GLOB 'wave2vec-fa-*'
    )
    OR (
      json_extract(data, '$.requested_engine') = 'cantonese_fa'
      AND engine_version NOT GLOB 'wav2vec-canto-*'
    )
  );

UPDATE cache_entries
SET engine_version = json_extract(data, '$.request_engine_identity.version')
WHERE task = 'forced_alignment_raw_evidence'
  AND json_valid(data)
  AND json_extract(data, '$.schema_version') = 2
  AND typeof(json_extract(data, '$.request_engine_identity.version')) = 'text'
  AND trim(json_extract(data, '$.request_engine_identity.version')) <> ''
  AND engine_version <> json_extract(data, '$.request_engine_identity.version');

DELETE FROM cache_entries
WHERE task = 'forced_alignment_raw_evidence'
  AND json_valid(data)
  AND json_extract(data, '$.schema_version') = 1
  AND (
    (
      json_extract(data, '$.requested_engine') = 'whisper_fa'
      AND engine_version NOT GLOB 'whisper-fa-*'
    )
    OR (
      json_extract(data, '$.requested_engine') = 'wav2vec_fa'
      AND engine_version NOT GLOB 'wave2vec-fa-*'
    )
    OR (
      json_extract(data, '$.requested_engine') = 'cantonese_fa'
      AND engine_version NOT GLOB 'wav2vec-canto-*'
    )
  );

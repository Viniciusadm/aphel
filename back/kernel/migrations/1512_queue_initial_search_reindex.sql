INSERT INTO jobs (id, queue, kind, payload, attempts, max_attempts, run_at, unique_key)
VALUES (UUID(), 'default', 'search.reindex', JSON_OBJECT(), 0, 5, UTC_TIMESTAMP(6), 'search.reindex:initial');

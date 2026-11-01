-- Add migration script here
WITH updated AS (
    UPDATE cloud_tracks
    SET document = jsonb_set(
        jsonb_set(
            document,
            '{added_at}',
            to_jsonb(extract(epoch FROM now())::bigint)
        ),
        '{revision}',
        to_jsonb(COALESCE((document->>'revision')::bigint, 0) + 1)
    )
    WHERE COALESCE((document->>'added_at')::bigint, 0) <= 0
    RETURNING user_id, document
)
INSERT INTO cloud_changes(user_id, value)
SELECT user_id, jsonb_build_object('kind', 'track', 'value', document)
FROM updated;

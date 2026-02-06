-- Revoke all existing OAuth2 refresh tokens that were stored as plaintext.
-- After this migration, the application stores HMAC-SHA256 hashes of refresh
-- tokens instead of plaintext values. Existing plaintext tokens cannot match
-- the new hashed lookups, so they are revoked to avoid orphaned rows.
-- Users will seamlessly re-authenticate and receive new (properly hashed) tokens.
UPDATE oauth2_refresh_tokens SET revoked = 1 WHERE revoked = 0;

-- OAuth client-side state storage for CSRF protection
-- Used when Pierre acts as an OAuth client (e.g., connecting to Strava, Fitbit)
-- Separate from oauth2_states which is for Pierre's OAuth2 server flow and has
-- FK constraints to oauth2_clients
CREATE TABLE IF NOT EXISTS oauth_client_states (
    state TEXT PRIMARY KEY,
    provider TEXT NOT NULL,
    user_id TEXT,
    tenant_id TEXT,
    redirect_uri TEXT NOT NULL,
    scope TEXT,
    pkce_code_verifier TEXT,
    created_at TEXT NOT NULL,
    expires_at TEXT NOT NULL,
    used INTEGER NOT NULL DEFAULT 0
);

CREATE INDEX IF NOT EXISTS idx_oauth_client_states_expires_at ON oauth_client_states(expires_at);
CREATE INDEX IF NOT EXISTS idx_oauth_client_states_provider ON oauth_client_states(provider);

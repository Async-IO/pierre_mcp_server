// ABOUTME: Typestate pattern for OAuth 2.0 flow with compile-time state transition safety
// ABOUTME: Invalid OAuth state transitions become compile errors, not runtime errors
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

use std::fmt::{Display, Formatter, Result as FmtResult};
use std::marker::PhantomData;

use chrono::{DateTime, Duration, Utc};
use uuid::Uuid;

use super::models::{AuthorizeResponse, OAuth2Error, TokenResponse};

// ============================================================================
// State Marker Types
// ============================================================================

/// Initial state: OAuth flow has not started
/// Valid transitions: -> Authorized (via authorization code grant)
#[derive(Debug)]
pub struct Initial;

/// Authorized state: Client has received an authorization code
/// Valid transitions: -> Authenticated (via token exchange)
#[derive(Debug)]
pub struct Authorized {
    /// The authorization code received from the authorization endpoint
    pub code: String,
    /// State parameter for CSRF protection
    pub state: Option<String>,
    /// PKCE code verifier (required when `code_challenge` was used)
    pub code_verifier: Option<String>,
    /// When the authorization code expires
    pub expires_at: DateTime<Utc>,
}

/// Authenticated state: Client has valid access and refresh tokens
/// Valid transitions: -> Refreshable (when access token expires)
#[derive(Debug)]
pub struct Authenticated {
    /// JWT access token
    pub access_token: String,
    /// Token type (always "Bearer")
    pub token_type: String,
    /// When the access token expires
    pub expires_at: DateTime<Utc>,
    /// Granted scopes
    pub scope: Option<String>,
    /// Refresh token for obtaining new access tokens
    pub refresh_token: Option<String>,
}

/// Refreshable state: Access token expired but refresh token is valid
/// Valid transitions: -> Authenticated (via refresh token grant)
#[derive(Debug)]
pub struct Refreshable {
    /// The refresh token for obtaining new tokens
    pub refresh_token: String,
    /// Original granted scopes
    pub scope: Option<String>,
}

// ============================================================================
// PKCE Configuration
// ============================================================================

/// PKCE (Proof Key for Code Exchange) configuration for enhanced security
#[derive(Debug, Clone)]
pub struct PkceConfig {
    /// The code challenge sent to the authorization endpoint
    pub code_challenge: String,
    /// The code challenge method (always S256 for security)
    pub code_challenge_method: PkceMethod,
    /// The original code verifier (kept secret until token exchange)
    pub code_verifier: String,
}

/// PKCE code challenge method
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PkceMethod {
    /// SHA-256 transformation (RFC 7636 required method)
    S256,
}

impl PkceMethod {
    /// Returns the string representation for OAuth requests
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::S256 => "S256",
        }
    }
}

impl Display for PkceMethod {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        write!(f, "{}", self.as_str())
    }
}

// ============================================================================
// OAuth Flow with Typestate
// ============================================================================

/// OAuth 2.0 flow with compile-time state machine enforcement
///
/// This struct uses the typestate pattern to ensure that OAuth flow
/// transitions are valid at compile time. Invalid transitions (e.g.,
/// calling `exchange_code` before `authorize`) will result in compile errors.
///
/// # Type Parameters
///
/// * `State` - The current state of the OAuth flow (Initial, Authorized, etc.)
///
/// # Example
///
/// ```no_run
/// use pierre_mcp_server::oauth2_server::{
///     AuthorizeResponse, Initial, OAuthFlow, TokenResponse,
/// };
///
/// fn example() -> Result<(), pierre_mcp_server::oauth2_server::OAuth2Error> {
///     // Start a new OAuth flow
///     let flow = OAuthFlow::<Initial>::new("client_123", "https://app.example.com/callback");
///
///     // Transition to Authorized state
///     let authorized = flow.authorize(AuthorizeResponse {
///         code: "abc".to_owned(),
///         state: None,
///     });
///
///     // Exchange code for tokens (only callable in Authorized state)
///     let token_response = TokenResponse {
///         access_token: "access_xyz".to_owned(),
///         token_type: "Bearer".to_owned(),
///         expires_in: 3600,
///         scope: Some("read".to_owned()),
///         refresh_token: Some("refresh_xyz".to_owned()),
///     };
///     let authenticated = authorized.exchange(token_response)?;
///
///     // Get access token (only callable in Authenticated state)
///     let token = authenticated.access_token();
///     println!("Access token: {token}");
///     Ok(())
/// }
/// ```
#[derive(Debug)]
pub struct OAuthFlow<State> {
    /// OAuth 2.0 client identifier
    client_id: String,
    /// Registered redirect URI
    redirect_uri: String,
    /// User identifier (set after authorization)
    user_id: Option<Uuid>,
    /// Tenant identifier for multi-tenancy
    tenant_id: Option<String>,
    /// PKCE configuration (optional but recommended)
    pkce: Option<PkceConfig>,
    /// Current state data
    state: State,
    /// Marker to prevent state from being dropped
    _marker: PhantomData<State>,
}

// ============================================================================
// Initial State Implementation
// ============================================================================

impl OAuthFlow<Initial> {
    /// Create a new OAuth flow in the Initial state
    ///
    /// # Arguments
    ///
    /// * `client_id` - The OAuth 2.0 client identifier
    /// * `redirect_uri` - The registered redirect URI
    #[must_use]
    pub fn new(client_id: impl Into<String>, redirect_uri: impl Into<String>) -> Self {
        Self {
            client_id: client_id.into(),
            redirect_uri: redirect_uri.into(),
            user_id: None,
            tenant_id: None,
            pkce: None,
            state: Initial,
            _marker: PhantomData,
        }
    }

    /// Create a new OAuth flow with PKCE support
    ///
    /// PKCE (Proof Key for Code Exchange) provides additional security
    /// for public clients that cannot securely store a client secret.
    ///
    /// # Arguments
    ///
    /// * `client_id` - The OAuth 2.0 client identifier
    /// * `redirect_uri` - The registered redirect URI
    /// * `pkce` - PKCE configuration with code verifier and challenge
    #[must_use]
    pub fn with_pkce(
        client_id: impl Into<String>,
        redirect_uri: impl Into<String>,
        pkce: PkceConfig,
    ) -> Self {
        Self {
            client_id: client_id.into(),
            redirect_uri: redirect_uri.into(),
            user_id: None,
            tenant_id: None,
            pkce: Some(pkce),
            state: Initial,
            _marker: PhantomData,
        }
    }

    /// Set the tenant identifier for multi-tenancy support
    #[must_use]
    pub fn with_tenant(mut self, tenant_id: impl Into<String>) -> Self {
        self.tenant_id = Some(tenant_id.into());
        self
    }

    /// Transition from Initial to Authorized state upon receiving an authorization code
    ///
    /// This method consumes the Initial state flow and returns an Authorized state flow.
    /// The transition is enforced at compile time - you cannot call this method twice
    /// or call it on a flow that's already in Authorized state.
    ///
    /// # Arguments
    ///
    /// * `response` - The authorization response containing the code and state
    ///
    /// # Returns
    ///
    /// A new `OAuthFlow<Authorized>` ready for token exchange
    #[must_use]
    pub fn authorize(self, response: AuthorizeResponse) -> OAuthFlow<Authorized> {
        let code_verifier = self.pkce.as_ref().map(|p| p.code_verifier.clone());
        // Authorization codes are valid for 10 minutes per RFC 6749
        let expires_at = Utc::now() + Duration::minutes(10);

        OAuthFlow {
            client_id: self.client_id,
            redirect_uri: self.redirect_uri,
            user_id: self.user_id,
            tenant_id: self.tenant_id,
            pkce: self.pkce,
            state: Authorized {
                code: response.code,
                state: response.state,
                code_verifier,
                expires_at,
            },
            _marker: PhantomData,
        }
    }

    /// Transition directly to Authorized state with a pre-existing authorization code
    ///
    /// Use this when you have an authorization code from an external source
    /// (e.g., OAuth callback) and need to resume the flow.
    #[must_use]
    pub fn with_authorization_code(
        self,
        code: impl Into<String>,
        state: Option<String>,
    ) -> OAuthFlow<Authorized> {
        let code_verifier = self.pkce.as_ref().map(|p| p.code_verifier.clone());
        let expires_at = Utc::now() + Duration::minutes(10);

        OAuthFlow {
            client_id: self.client_id,
            redirect_uri: self.redirect_uri,
            user_id: self.user_id,
            tenant_id: self.tenant_id,
            pkce: self.pkce,
            state: Authorized {
                code: code.into(),
                state,
                code_verifier,
                expires_at,
            },
            _marker: PhantomData,
        }
    }
}

// ============================================================================
// Authorized State Implementation
// ============================================================================

impl OAuthFlow<Authorized> {
    /// Get the authorization code for token exchange
    #[must_use]
    pub fn code(&self) -> &str {
        &self.state.code
    }

    /// Get the state parameter (if provided)
    #[must_use]
    pub fn state_param(&self) -> Option<&str> {
        self.state.state.as_deref()
    }

    /// Get the PKCE code verifier for token exchange
    #[must_use]
    pub fn code_verifier(&self) -> Option<&str> {
        self.state.code_verifier.as_deref()
    }

    /// Check if the authorization code has expired
    #[must_use]
    pub fn is_code_expired(&self) -> bool {
        Utc::now() > self.state.expires_at
    }

    /// Set the user ID after authorization
    #[must_use]
    pub const fn with_user(mut self, user_id: Uuid) -> Self {
        self.user_id = Some(user_id);
        self
    }

    /// Transition from Authorized to Authenticated state upon successful token exchange
    ///
    /// This method validates the token response and transitions to the Authenticated state.
    /// The transition is enforced at compile time.
    ///
    /// # Arguments
    ///
    /// * `response` - The token response containing access token and optionally refresh token
    ///
    /// # Errors
    ///
    /// Returns `OAuth2Error` if the authorization code has expired
    pub fn exchange(
        self,
        response: TokenResponse,
    ) -> Result<OAuthFlow<Authenticated>, OAuth2Error> {
        // Validate authorization code hasn't expired
        if self.is_code_expired() {
            return Err(OAuth2Error::invalid_grant("Authorization code has expired"));
        }

        let expires_at = Utc::now() + Duration::seconds(response.expires_in);

        Ok(OAuthFlow {
            client_id: self.client_id,
            redirect_uri: self.redirect_uri,
            user_id: self.user_id,
            tenant_id: self.tenant_id,
            pkce: self.pkce,
            state: Authenticated {
                access_token: response.access_token,
                token_type: response.token_type,
                expires_at,
                scope: response.scope,
                refresh_token: response.refresh_token,
            },
            _marker: PhantomData,
        })
    }
}

// ============================================================================
// Authenticated State Implementation
// ============================================================================

impl OAuthFlow<Authenticated> {
    /// Get the access token for API requests
    #[must_use]
    pub fn access_token(&self) -> &str {
        &self.state.access_token
    }

    /// Get the token type (always "Bearer")
    #[must_use]
    pub fn token_type(&self) -> &str {
        &self.state.token_type
    }

    /// Get the granted scopes
    #[must_use]
    pub fn scope(&self) -> Option<&str> {
        self.state.scope.as_deref()
    }

    /// Get the refresh token (if available)
    #[must_use]
    pub fn refresh_token(&self) -> Option<&str> {
        self.state.refresh_token.as_deref()
    }

    /// Check if the access token has expired
    #[must_use]
    pub fn is_token_expired(&self) -> bool {
        Utc::now() > self.state.expires_at
    }

    /// Get the expiration time of the access token
    #[must_use]
    pub const fn expires_at(&self) -> DateTime<Utc> {
        self.state.expires_at
    }

    /// Get seconds until token expiration (negative if expired)
    #[must_use]
    pub fn expires_in_seconds(&self) -> i64 {
        let remaining = self.state.expires_at - Utc::now();
        remaining.num_seconds()
    }

    /// Transition from Authenticated to Refreshable state when access token expires
    ///
    /// This method is only callable when the access token has expired and a
    /// refresh token is available. If no refresh token exists, the flow
    /// must be restarted from Initial state.
    ///
    /// # Errors
    ///
    /// Returns `OAuth2Error` if:
    /// - The access token has not yet expired
    /// - No refresh token is available
    pub fn needs_refresh(self) -> Result<OAuthFlow<Refreshable>, OAuth2Error> {
        if !self.is_token_expired() {
            return Err(OAuth2Error::invalid_request(
                "Access token has not expired yet",
            ));
        }

        let refresh_token = self.state.refresh_token.ok_or_else(|| {
            OAuth2Error::invalid_grant("No refresh token available; re-authentication required")
        })?;

        Ok(OAuthFlow {
            client_id: self.client_id,
            redirect_uri: self.redirect_uri,
            user_id: self.user_id,
            tenant_id: self.tenant_id,
            pkce: self.pkce,
            state: Refreshable {
                refresh_token,
                scope: self.state.scope,
            },
            _marker: PhantomData,
        })
    }

    /// Force transition to Refreshable state regardless of expiration
    ///
    /// Use this when you want to proactively refresh tokens before they expire.
    ///
    /// # Errors
    ///
    /// Returns `OAuth2Error` if no refresh token is available
    pub fn force_refresh(self) -> Result<OAuthFlow<Refreshable>, OAuth2Error> {
        let refresh_token = self.state.refresh_token.ok_or_else(|| {
            OAuth2Error::invalid_grant("No refresh token available; re-authentication required")
        })?;

        Ok(OAuthFlow {
            client_id: self.client_id,
            redirect_uri: self.redirect_uri,
            user_id: self.user_id,
            tenant_id: self.tenant_id,
            pkce: self.pkce,
            state: Refreshable {
                refresh_token,
                scope: self.state.scope,
            },
            _marker: PhantomData,
        })
    }
}

// ============================================================================
// Refreshable State Implementation
// ============================================================================

impl OAuthFlow<Refreshable> {
    /// Get the refresh token for obtaining new tokens
    #[must_use]
    pub fn refresh_token(&self) -> &str {
        &self.state.refresh_token
    }

    /// Get the original granted scopes
    #[must_use]
    pub fn scope(&self) -> Option<&str> {
        self.state.scope.as_deref()
    }

    /// Transition from Refreshable back to Authenticated state after token refresh
    ///
    /// This method consumes the refresh token and returns a new Authenticated state
    /// with fresh tokens. Note that the refresh token in the response may be different
    /// from the original (token rotation).
    ///
    /// # Arguments
    ///
    /// * `response` - The token response containing new access and refresh tokens
    #[must_use]
    pub fn refresh(self, response: TokenResponse) -> OAuthFlow<Authenticated> {
        let expires_at = Utc::now() + Duration::seconds(response.expires_in);

        OAuthFlow {
            client_id: self.client_id,
            redirect_uri: self.redirect_uri,
            user_id: self.user_id,
            tenant_id: self.tenant_id,
            pkce: self.pkce,
            state: Authenticated {
                access_token: response.access_token,
                token_type: response.token_type,
                expires_at,
                scope: response.scope.or(self.state.scope),
                refresh_token: response.refresh_token,
            },
            _marker: PhantomData,
        }
    }
}

// ============================================================================
// Common Accessors (Available in All States)
// ============================================================================

impl<State> OAuthFlow<State> {
    /// Get the OAuth 2.0 client identifier
    #[must_use]
    pub fn client_id(&self) -> &str {
        &self.client_id
    }

    /// Get the registered redirect URI
    #[must_use]
    pub fn redirect_uri(&self) -> &str {
        &self.redirect_uri
    }

    /// Get the user identifier (if set)
    #[must_use]
    pub const fn user_id(&self) -> Option<Uuid> {
        self.user_id
    }

    /// Get the tenant identifier (if set)
    #[must_use]
    pub fn tenant_id(&self) -> Option<&str> {
        self.tenant_id.as_deref()
    }

    /// Get the PKCE configuration (if configured)
    #[must_use]
    pub const fn pkce_config(&self) -> Option<&PkceConfig> {
        match &self.pkce {
            Some(p) => Some(p),
            None => None,
        }
    }
}

// ============================================================================
// Builder for PKCE Configuration
// ============================================================================

impl PkceConfig {
    /// Create a new PKCE configuration
    ///
    /// # Arguments
    ///
    /// * `code_verifier` - The randomly generated code verifier (43-128 characters)
    /// * `code_challenge` - The S256-transformed code challenge
    #[must_use]
    pub fn new(code_verifier: impl Into<String>, code_challenge: impl Into<String>) -> Self {
        Self {
            code_verifier: code_verifier.into(),
            code_challenge: code_challenge.into(),
            code_challenge_method: PkceMethod::S256,
        }
    }

    /// Get the code challenge for the authorization request
    #[must_use]
    pub fn code_challenge(&self) -> &str {
        &self.code_challenge
    }

    /// Get the code challenge method
    #[must_use]
    pub const fn code_challenge_method(&self) -> PkceMethod {
        self.code_challenge_method
    }

    /// Get the code verifier for token exchange
    #[must_use]
    pub fn code_verifier(&self) -> &str {
        &self.code_verifier
    }
}

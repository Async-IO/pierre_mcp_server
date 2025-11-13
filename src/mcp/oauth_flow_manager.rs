// ABOUTME: OAuth template rendering for multi-tenant MCP server
// ABOUTME: Provides HTML template rendering for OAuth success and error pages
//
// Licensed under either of Apache License, Version 2.0 or MIT License at your option.
// Copyright ©2025 Async-IO.org

/// Template renderer for OAuth success and error pages
pub struct OAuthTemplateRenderer;

impl OAuthTemplateRenderer {
    /// Render OAuth success template
    ///
    /// # Errors
    /// Returns an error if template formatting fails
    pub fn render_success_template(
        provider: &str,
        callback_response: &crate::routes::OAuthCallbackResponse,
    ) -> Result<String, Box<dyn std::error::Error>> {
        const TEMPLATE: &str = include_str!("../../templates/oauth_success.html");

        let rendered = TEMPLATE
            .replace("{{PROVIDER}}", provider)
            .replace("{{USER_ID}}", &callback_response.user_id);

        Ok(rendered)
    }

    /// Render OAuth error template
    ///
    /// # Errors
    /// Returns an error if template formatting fails
    pub fn render_error_template(
        provider: &str,
        error: &str,
        description: Option<&str>,
    ) -> Result<String, Box<dyn std::error::Error>> {
        const TEMPLATE: &str = include_str!("../../templates/oauth_error.html");

        let description_html = description
            .map(|d| format!("<div class=\"description\"><strong>Description:</strong> {d}</div>"))
            .unwrap_or_default();

        let rendered = TEMPLATE
            .replace("{{PROVIDER}}", provider)
            .replace("{{ERROR}}", error)
            .replace("{{DESCRIPTION}}", &description_html);

        Ok(rendered)
    }
}

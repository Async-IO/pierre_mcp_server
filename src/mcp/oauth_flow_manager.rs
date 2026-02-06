// ABOUTME: OAuth template rendering for multi-tenant MCP server
// ABOUTME: Provides HTML template rendering for OAuth success and error pages
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

use html_escape::encode_text;

use crate::types::OAuthCallbackResponse;

/// Template renderer for OAuth success and error pages
pub struct OAuthTemplateRenderer;

impl OAuthTemplateRenderer {
    /// Render OAuth success template
    #[must_use]
    pub fn render_success_template(
        provider: &str,
        callback_response: &OAuthCallbackResponse,
    ) -> String {
        const TEMPLATE: &str = include_str!("../../templates/oauth_success.html");

        // Capitalize provider name for display (e.g., "strava" -> "Strava")
        let capitalized_provider = Self::capitalize_provider(provider);

        // Escape all interpolated values to prevent XSS
        TEMPLATE
            .replace("{{PROVIDER}}", &encode_text(&capitalized_provider))
            .replace("{{PROVIDER_LOWER}}", &encode_text(&provider.to_lowercase()))
            .replace("{{USER_ID}}", &encode_text(&callback_response.user_id))
    }

    /// Capitalize provider name for display
    fn capitalize_provider(provider: &str) -> String {
        let mut chars = provider.chars();
        chars.next().map_or_else(String::new, |first| {
            first.to_uppercase().collect::<String>() + chars.as_str()
        })
    }

    /// Render OAuth error template
    #[must_use]
    pub fn render_error_template(provider: &str, error: &str, description: Option<&str>) -> String {
        const TEMPLATE: &str = include_str!("../../templates/oauth_error.html");

        // Escape description text before wrapping in HTML structure
        let description_html = description
            .map(|d| {
                let escaped = encode_text(d);
                format!("<div class=\"description\"><strong>Description:</strong> {escaped}</div>")
            })
            .unwrap_or_default();

        // Escape provider and error values to prevent XSS
        TEMPLATE
            .replace("{{PROVIDER}}", &encode_text(provider))
            .replace("{{ERROR}}", &encode_text(error))
            .replace("{{DESCRIPTION}}", &description_html)
    }
}

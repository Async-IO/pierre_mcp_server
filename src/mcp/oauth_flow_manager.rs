// ABOUTME: OAuth template rendering for multi-tenant MCP server
// ABOUTME: Provides HTML template rendering for OAuth success and error pages
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

use crate::routes::OAuthCallbackResponse;
use std::error::Error;

/// Template renderer for OAuth success and error pages
pub struct OAuthTemplateRenderer;

impl OAuthTemplateRenderer {
    /// Render OAuth success template
    ///
    /// # Errors
    /// Returns an error if template formatting fails
    pub fn render_success_template(
        provider: &str,
        callback_response: &OAuthCallbackResponse,
    ) -> Result<String, Box<dyn Error>> {
        const TEMPLATE: &str = include_str!("../../templates/oauth_success.html");

        // Capitalize provider name for display (e.g., "strava" -> "Strava")
        let capitalized_provider = Self::capitalize_provider(provider);

        let rendered = TEMPLATE
            .replace("{{PROVIDER}}", &capitalized_provider)
            .replace("{{PROVIDER_LOWER}}", &provider.to_lowercase())
            .replace("{{USER_ID}}", &callback_response.user_id);

        Ok(rendered)
    }

    /// Capitalize provider name for display
    fn capitalize_provider(provider: &str) -> String {
        let mut chars = provider.chars();
        chars.next().map_or_else(String::new, |first| {
            first.to_uppercase().collect::<String>() + chars.as_str()
        })
    }

    /// Render OAuth error template
    ///
    /// # Errors
    /// Returns an error if template formatting fails
    pub fn render_error_template(
        provider: &str,
        error: &str,
        description: Option<&str>,
    ) -> Result<String, Box<dyn Error>> {
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

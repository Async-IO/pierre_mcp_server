// ABOUTME: HTML escaping utilities to prevent XSS in server-rendered templates
// ABOUTME: Provides attribute-safe escaping for values injected into HTML templates
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

/// Escape a string for safe insertion into HTML attribute values.
///
/// Replaces the five HTML-special characters (`&`, `<`, `>`, `"`, `'`) with their
/// corresponding HTML entities. This prevents attribute breakout and script injection
/// when inserting user-controlled values into HTML attributes like `value="..."`.
#[must_use]
pub fn escape_html_attribute(input: &str) -> String {
    let mut output = String::with_capacity(input.len());
    for ch in input.chars() {
        match ch {
            '&' => output.push_str("&amp;"),
            '<' => output.push_str("&lt;"),
            '>' => output.push_str("&gt;"),
            '"' => output.push_str("&quot;"),
            '\'' => output.push_str("&#x27;"),
            _ => output.push(ch),
        }
    }
    output
}

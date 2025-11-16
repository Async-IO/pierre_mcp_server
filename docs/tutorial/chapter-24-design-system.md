# chapter 24: design system - templates, frontend & UX

This chapter covers Pierre's design system including OAuth templates, frontend architecture, and user experience patterns for fitness data visualization and interaction.

## what you'll learn

- OAuth success/error templates
- Template rendering system
- HTML/CSS design patterns
- User feedback mechanisms
- Responsive design for fitness data
- Error handling UX

## OAuth templates

Pierre uses HTML templates for OAuth callback pages.

**OAuth success template** (templates/oauth_success.html):
```html
<!DOCTYPE html>
<html>
<head>
    <title>OAuth Success - Pierre Fitness</title>
    <style>
        body {
            font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto;
            display: flex;
            justify-content: center;
            align-items: center;
            height: 100vh;
            margin: 0;
            background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
        }
        .container {
            background: white;
            padding: 40px;
            border-radius: 12px;
            box-shadow: 0 10px 40px rgba(0,0,0,0.2);
            text-align: center;
        }
        h1 { color: #667eea; }
        .success-icon { font-size: 64px; color: #10b981; }
    </style>
</head>
<body>
    <div class="container">
        <div class="success-icon">✓</div>
        <h1>Successfully Connected to {{PROVIDER}}</h1>
        <p>You can now close this window and return to the app.</p>
        <p>User ID: {{USER_ID}}</p>
    </div>
</body>
</html>
```

**Template rendering**:

**Source**: src/oauth2_client/flow_manager.rs:11-26
```rust
pub struct OAuthTemplateRenderer;

impl OAuthTemplateRenderer {
    pub fn render_success_template(
        provider: &str,
        callback_response: &OAuthCallbackResponse,
    ) -> Result<String, Box<dyn std::error::Error>> {
        const TEMPLATE: &str = include_str!("../../templates/oauth_success.html");

        let rendered = TEMPLATE
            .replace("{{PROVIDER}}", provider)
            .replace("{{USER_ID}}", &callback_response.user_id);

        Ok(rendered)
    }
}
```

## key takeaways

1. **OAuth templates**: HTML templates for success/error feedback.
2. **Template rendering**: String replacement with `{{PLACEHOLDER}}` syntax.
3. **Responsive design**: Mobile-first, gradient backgrounds, card layouts.
4. **User feedback**: Clear success/error states with visual indicators.

---

**Next Chapter**: [Chapter 25: Production Deployment, Clippy & Performance](./chapter-25-deployment.md) - Learn about production deployment strategies, Clippy lint configuration, performance optimization, and monitoring.

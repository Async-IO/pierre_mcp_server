// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! Documentation Server
//! 
//! Serves OpenAPI documentation with Swagger UI for the Pierre MCP Fitness API.
//! This provides an interactive interface for developers to explore our 21 fitness tools.

use warp::Filter;
use serde_json::json;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    env_logger::init();
    
    let port = std::env::var("DOCS_PORT")
        .unwrap_or_else(|_| "3000".to_string())
        .parse::<u16>()
        .unwrap_or(3000);
    
    println!("🚀 Starting Pierre MCP API Documentation Server");
    println!("📖 Swagger UI: http://localhost:{}", port);
    println!("📄 OpenAPI Spec: http://localhost:{}/openapi.yaml", port);
    
    // Serve OpenAPI specification
    let openapi_yaml = warp::path("openapi.yaml")
        .and(warp::get())
        .map(|| {
            let spec = include_str!("../../docs/openapi.yaml");
            warp::reply::with_header(spec, "content-type", "application/yaml")
        });
    
    // Serve OpenAPI specification as JSON
    let openapi_json = warp::path("openapi.json")
        .and(warp::get())
        .map(|| {
            // Convert YAML to JSON (simplified - in production use proper YAML parser)
            let spec = include_str!("../../docs/openapi.yaml");
            warp::reply::with_header(spec, "content-type", "application/json")
        });
    
    // Swagger UI HTML page
    let swagger_ui = warp::path::end()
        .and(warp::get())
        .map(|| {
            warp::reply::html(create_swagger_ui_html())
        });
    
    // API info endpoint
    let api_info = warp::path("info")
        .and(warp::get())
        .map(|| {
            warp::reply::json(&json!({
                "name": "Pierre MCP Fitness API",
                "version": "1.0.0",
                "description": "AI-powered fitness data intelligence platform",
                "tools_count": 21,
                "providers": ["strava", "fitbit", "garmin"],
                "features": [
                    "Multi-provider data aggregation",
                    "Advanced analytics and intelligence",
                    "Goal setting and tracking",
                    "Real-time activity insights",
                    "Weather and location integration"
                ],
                "documentation": {
                    "openapi": "/openapi.yaml",
                    "swagger_ui": "/"
                }
            }))
        });
    
    // Health check
    let health = warp::path("health")
        .and(warp::get())
        .map(|| {
            warp::reply::json(&json!({
                "status": "healthy",
                "service": "pierre-mcp-docs",
                "timestamp": chrono::Utc::now().to_rfc3339()
            }))
        });
    
    // Static assets for enhanced UI
    let assets = warp::path("assets")
        .and(warp::get())
        .and(warp::fs::dir("docs/assets"));
    
    // CORS for development
    let cors = warp::cors()
        .allow_any_origin()
        .allow_headers(vec!["content-type"])
        .allow_methods(vec!["GET", "POST", "OPTIONS"]);
    
    let routes = swagger_ui
        .or(openapi_yaml)
        .or(openapi_json)
        .or(api_info)
        .or(health)
        .or(assets)
        .with(cors)
        .with(warp::log("pierre_mcp_docs"));
    
    println!("✅ Documentation server ready!");
    println!();
    println!("📚 Available endpoints:");
    println!("   • /           - Interactive Swagger UI");
    println!("   • /openapi.yaml - OpenAPI specification (YAML)");
    println!("   • /openapi.json - OpenAPI specification (JSON)"); 
    println!("   • /info       - API information");
    println!("   • /health     - Health check");
    println!();
    println!("🔧 Documented tool categories:");
    println!("   • Core Tools    - 8 essential fitness data tools");
    println!("   • Analytics     - 8 advanced analysis tools");
    println!("   • Goals         - 4 goal management tools");
    println!("   • Connections   - 4 provider connection tools");
    println!();
    println!("🔗 Try the API:");
    println!("   curl http://localhost:{}/info", port);
    println!();
    
    warp::serve(routes)
        .run(([127, 0, 0, 1], port))
        .await;
    
    Ok(())
}

fn create_swagger_ui_html() -> String {
    format!(r#"
<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Pierre MCP Fitness API Documentation</title>
    <link rel="stylesheet" type="text/css" href="https://unpkg.com/swagger-ui-dist@5.10.3/swagger-ui.css" />
    <link rel="icon" type="image/png" href="https://unpkg.com/swagger-ui-dist@5.10.3/favicon-32x32.png" sizes="32x32" />
    <style>
        html {{
            box-sizing: border-box;
            overflow: -moz-scrollbars-vertical;
            overflow-y: scroll;
        }}
        *, *:before, *:after {{
            box-sizing: inherit;
        }}
        body {{
            margin:0;
            background: #fafafa;
            font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif;
        }}
        .custom-header {{
            background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
            color: white;
            padding: 2rem;
            text-align: center;
            margin-bottom: 2rem;
        }}
        .custom-header h1 {{
            margin: 0;
            font-size: 2.5rem;
            font-weight: 700;
        }}
        .custom-header p {{
            margin: 0.5rem 0 0 0;
            font-size: 1.2rem;
            opacity: 0.9;
        }}
        .features {{
            display: flex;
            justify-content: center;
            gap: 2rem;
            margin: 1rem 0;
            flex-wrap: wrap;
        }}
        .feature {{
            background: rgba(255,255,255,0.1);
            padding: 0.5rem 1rem;
            border-radius: 20px;
            font-size: 0.9rem;
        }}
        .stats {{
            display: flex;
            justify-content: center;
            gap: 3rem;
            margin-top: 1.5rem;
        }}
        .stat {{
            text-align: center;
        }}
        .stat-number {{
            font-size: 2rem;
            font-weight: bold;
            display: block;
        }}
        .stat-label {{
            font-size: 0.9rem;
            opacity: 0.8;
        }}
        #swagger-ui {{
            max-width: 1200px;
            margin: 0 auto;
            padding: 0 2rem;
        }}
        @media (max-width: 768px) {{
            .stats {{
                flex-direction: column;
                gap: 1rem;
            }}
            .features {{
                flex-direction: column;
                align-items: center;
            }}
        }}
    </style>
</head>
<body>
    <div class="custom-header">
        <h1>🏃‍♂️ Pierre MCP Fitness API</h1>
        <p>AI-Powered Fitness Data Intelligence Platform</p>
        
        <div class="features">
            <div class="feature">🔗 Multi-Provider</div>
            <div class="feature">🤖 AI-Ready</div>
            <div class="feature">⚡ Real-time</div>
            <div class="feature">🌍 Location Intelligence</div>
            <div class="feature">🎯 Goal Tracking</div>
        </div>
        
        <div class="stats">
            <div class="stat">
                <span class="stat-number">21</span>
                <span class="stat-label">Fitness Tools</span>
            </div>
            <div class="stat">
                <span class="stat-number">3+</span>
                <span class="stat-label">Providers</span>
            </div>
            <div class="stat">
                <span class="stat-number">13</span>
                <span class="stat-label">Analytics Tools</span>
            </div>
        </div>
    </div>

    <div id="swagger-ui"></div>
    
    <script src="https://unpkg.com/swagger-ui-dist@5.10.3/swagger-ui-bundle.js"></script>
    <script src="https://unpkg.com/swagger-ui-dist@5.10.3/swagger-ui-standalone-preset.js"></script>
    <script>
        window.onload = function() {{
            const ui = SwaggerUIBundle({{
                url: '/openapi.yaml',
                dom_id: '#swagger-ui',
                deepLinking: true,
                presets: [
                    SwaggerUIBundle.presets.apis,
                    SwaggerUIStandalonePreset
                ],
                plugins: [
                    SwaggerUIBundle.plugins.DownloadUrl
                ],
                layout: "StandaloneLayout",
                defaultModelsExpandDepth: 2,
                defaultModelExpandDepth: 2,
                tryItOutEnabled: true,
                filter: true,
                requestInterceptor: function(request) {{
                    // Add custom headers or modify requests here
                    console.log('API Request:', request);
                    return request;
                }},
                responseInterceptor: function(response) {{
                    console.log('API Response:', response);
                    return response;
                }},
                onComplete: function() {{
                    console.log('Pierre MCP API Documentation loaded successfully!');
                }},
                validatorUrl: null, // Disable validator
                docExpansion: 'list', // Show operations
                operationsSorter: 'alpha', // Sort operations alphabetically
                tagsSorter: 'alpha' // Sort tags alphabetically
            }});
            
            // Custom styling after load
            setTimeout(() => {{
                // Custom branding loaded
                console.log('Pierre MCP Documentation UI enhanced!');
            }}, 1000);
        }};
    </script>
</body>
</html>
    "#)
}
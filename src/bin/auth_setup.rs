// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use anyhow::Result;
use clap::{Parser, Subcommand};
use pierre_mcp_server::providers::strava::StravaProvider;
use pierre_mcp_server::providers::{FitnessProvider, AuthData};
use pierre_mcp_server::config::Config;
use pierre_mcp_server::constants::{env_config, oauth};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::net::TcpListener;
use tokio::io::AsyncWriteExt;
use tracing::{info, error};
use url::Url;
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Parser)]
#[command(name = "auth-setup")]
#[command(about = "Set up OAuth2 authentication for fitness providers")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Set up Strava OAuth2 authentication
    Strava {
        /// OAuth2 client ID
        #[arg(long)]
        client_id: String,
        
        /// OAuth2 client secret
        #[arg(long)]
        client_secret: String,
        
        /// Callback port (default: from env or 8080)
        #[arg(long)]
        port: Option<u16>,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();
    
    let cli = Cli::parse();
    
    match cli.command {
        Commands::Strava { client_id, client_secret, port } => {
            let port = port.unwrap_or_else(env_config::mcp_port);
            setup_strava_auth(client_id, client_secret, port).await?;
        }
    }
    
    Ok(())
}

async fn setup_strava_auth(client_id: String, client_secret: String, port: u16) -> Result<()> {
    info!("Setting up Strava OAuth2 authentication...");
    
    let mut provider = StravaProvider::new();
    provider.authenticate(AuthData::OAuth2 {
        client_id: client_id.clone(),
        client_secret: client_secret.clone(),
        access_token: None,
        refresh_token: None,
    }).await?;
    
    let redirect_uri = format!("http://localhost:{}/callback", port);
    let state = uuid::Uuid::new_v4().to_string();
    let auth_url = provider.get_auth_url(&redirect_uri, &state)?;
    
    println!("\nPlease visit this URL to authorize the application:");
    println!("{}\n", auth_url);
    
    // Start callback server
    let auth_code = Arc::new(Mutex::new(None));
    let auth_code_clone = auth_code.clone();
    
    let listener = TcpListener::bind(format!("127.0.0.1:{}", port)).await?;
    info!("Listening for OAuth callback on port {}", port);
    
    let handle = tokio::spawn(async move {
        if let Ok((socket, _)) = listener.accept().await {
            let (reader, mut writer) = socket.into_split();
            let mut reader = BufReader::new(reader);
            let mut line = String::new();
            
            if reader.read_line(&mut line).await.unwrap_or(0) > 0 {
                // Parse the GET request
                if let Some(path) = line.split_whitespace().nth(1) {
                    if let Ok(url) = Url::parse(&format!("http://localhost{}", path)) {
                        let params: std::collections::HashMap<_, _> = url.query_pairs().collect();
                        
                        if let Some(code) = params.get("code") {
                            *auth_code_clone.lock().await = Some(code.to_string());
                            
                            let response = "HTTP/1.1 200 OK\r\n\
                                Content-Type: text/html\r\n\r\n\
                                <html><body>\
                                <h1>Authorization successful!</h1>\
                                <p>You can close this window and return to the terminal.</p>\
                                </body></html>";
                            
                            writer.write_all(response.as_bytes()).await.ok();
                        }
                    }
                }
            }
        }
    });
    
    // Wait for auth code
    tokio::time::sleep(tokio::time::Duration::from_secs(60)).await;
    handle.abort();
    
    if let Some(code) = auth_code.lock().await.take() {
        info!("Received authorization code, exchanging for tokens...");
        
        let (access_token, refresh_token) = provider.exchange_code(&code).await?;
        
        // Update config
        let mut config = Config::load(None).unwrap_or_else(|_| Config {
            providers: std::collections::HashMap::new(),
        });
        
        config.providers.insert("strava".to_string(), pierre_mcp_server::config::ProviderConfig {
            auth_type: "oauth2".to_string(),
            client_id: Some(client_id),
            client_secret: Some(client_secret),
            access_token: Some(access_token),
            refresh_token: Some(refresh_token),
            api_key: None,
            redirect_uri: None,
            scopes: Some(oauth::STRAVA_DEFAULT_SCOPES.split(',').map(|s| s.to_string()).collect()),
        });
        
        config.save(None)?;
        
        println!("\n✅ Strava authentication successful!");
        println!("Configuration saved. You can now run the MCP server.");
    } else {
        error!("No authorization code received within timeout");
        return Err(anyhow::anyhow!("OAuth2 authorization failed"));
    }
    
    Ok(())
}
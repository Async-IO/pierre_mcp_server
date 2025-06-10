// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use anyhow::Result;
use clap::Parser;
use tracing::info;
use tracing_subscriber;

mod mcp;
mod providers;
mod models;
mod config;
mod oauth2_client;
mod intelligence;

use mcp::McpServer;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(short, long, default_value = "8080")]
    port: u16,
    
    #[arg(short, long)]
    config: Option<String>,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();
    
    let args = Args::parse();
    
    info!("Starting Fitness MCP Server on port {}", args.port);
    
    let config = config::Config::load(args.config)?;
    let server = McpServer::new(config);
    
    server.run(args.port).await?;
    
    Ok(())
}
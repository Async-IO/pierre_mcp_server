//! Demo showing database backend logging

use pierre_mcp_server::database_plugins::factory::Database;
use tracing_subscriber;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    println!("🔧 Pierre MCP Server Database Backend Logging Demo");
    println!("==================================================");

    // Demo 1: SQLite initialization
    println!("\n1️⃣  Initializing SQLite database:");
    let encryption_key = (0..32).collect::<Vec<u8>>();
    let sqlite_db = Database::new("sqlite::memory:", encryption_key.clone()).await?;
    
    println!("   Backend: {}", sqlite_db.backend_info());
    println!("   Type: {:?}", sqlite_db.database_type());
    println!("\n   Detailed info:");
    println!("{}", sqlite_db.info_summary());

    #[cfg(feature = "postgresql")]
    {
        println!("\n2️⃣  Would initialize PostgreSQL database (if connection available):");
        // Note: This would fail without a real PostgreSQL instance, so we just show what would happen
        println!("   Command: Database::new(\"postgresql://user:pass@host/db\", encryption_key)");
        println!("   Expected logs:");
        println!("   🗄️  Detected database type: PostgreSQL");
        println!("   🐘 Initializing PostgreSQL database");
        println!("   ✅ PostgreSQL database initialized successfully");
        println!("   Backend: PostgreSQL (Cloud-Ready)");
    }

    #[cfg(not(feature = "postgresql"))]
    {
        println!("\n2️⃣  PostgreSQL feature not enabled in this build");
        println!("   To enable: cargo run --features postgresql --example database_logging_demo");
    }

    // Demo 3: Health check information
    println!("\n3️⃣  Health check would return:");
    println!("   {{");
    println!("     \"backend\": \"{:?}\",", sqlite_db.database_type());
    println!("     \"backend_info\": \"{}\",", sqlite_db.backend_info());
    println!("     \"query_duration_ms\": \"<measured>\",");
    println!("     \"status\": \"connected\",");
    println!("     \"user_count\": \"<actual_count>\"");
    println!("   }}");

    println!("\n✅ Demo completed successfully!");
    Ok(())
}
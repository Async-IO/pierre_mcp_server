use anyhow::Result;
use pierre_mcp_server::database::Database;

/// Create a test database instance
///
/// # Errors
///
/// Returns an error if database initialization fails
pub async fn create_test_db() -> Result<Database> {
    // Use a simple in-memory database - each connection gets its own isolated instance
    let database_url = "sqlite::memory:";
    Database::new(database_url, vec![0u8; 32]).await
}

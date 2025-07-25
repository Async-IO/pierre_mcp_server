use pierre_mcp_server::database_plugins::factory::{detect_database_type, DatabaseType};

#[test]
fn test_detect_database_type() {
    // SQLite URLs
    assert_eq!(
        detect_database_type("sqlite:./data/test.db").unwrap(),
        DatabaseType::SQLite
    );
    assert_eq!(
        detect_database_type("sqlite::memory:").unwrap(),
        DatabaseType::SQLite
    );

    // PostgreSQL URLs (only test detection, not creation)
    #[cfg(feature = "postgresql")]
    {
        assert_eq!(
            detect_database_type("postgresql://user:pass@localhost/db").unwrap(),
            DatabaseType::PostgreSQL
        );
        assert_eq!(
            detect_database_type("postgres://user:pass@localhost/db").unwrap(),
            DatabaseType::PostgreSQL
        );
    }

    // Invalid URLs
    assert!(detect_database_type("mysql://user:pass@localhost/db").is_err());
    assert!(detect_database_type("invalid_url").is_err());
}

// ABOUTME: JWKS (JSON Web Key Set) initialization helpers for pierre-cli
// ABOUTME: Manages RSA keypair loading and generation for JWT signing
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

use pierre_mcp_server::{
    admin::jwks::JwksManager,
    database_plugins::{factory::Database, DatabaseProvider},
    errors::AppError,
};
use tracing::{info, warn};

/// Generate a new RSA keypair and persist it to the database
pub async fn generate_and_persist_keypair(
    database: &Database,
    jwks_manager: &mut JwksManager,
) -> Result<(), AppError> {
    info!("No persisted RSA keys found, generating new keypair");
    let kid = format!("key_{}", chrono::Utc::now().format("%Y%m%d_%H%M%S"));
    jwks_manager.generate_rsa_key_pair(&kid)?;

    let key_pair = jwks_manager.get_active_key()?;
    let private_pem = key_pair.export_private_key_pem()?;
    let public_pem = key_pair.export_public_key_pem()?;
    let created_at = chrono::Utc::now();
    database
        .save_rsa_keypair(&kid, &private_pem, &public_pem, created_at, true, 4096)
        .await?;
    info!("Generated and persisted new RSA keypair: {}", kid);
    Ok(())
}

/// Load existing RSA keypairs from database into JWKS manager
pub fn load_existing_keypairs(
    jwks_manager: &mut JwksManager,
    keypairs: Vec<(String, String, String, chrono::DateTime<chrono::Utc>, bool)>,
) -> Result<(), AppError> {
    info!(
        "Loading {} persisted RSA keypairs from database",
        keypairs.len()
    );
    jwks_manager.load_keys_from_database(keypairs)?;
    info!("Successfully loaded RSA keys from database");
    Ok(())
}

/// Generate ephemeral keys when database fails
pub fn generate_ephemeral_keys(
    jwks_manager: &mut JwksManager,
    error: &AppError,
) -> Result<(), AppError> {
    warn!("Failed to load RSA keys from database: {error}. Generating ephemeral keys.");
    jwks_manager.generate_rsa_key_pair("admin_key_ephemeral")?;
    Ok(())
}

/// Initialize JWKS manager by loading keys from database or generating new ones
/// This ensures the CLI uses the same RSA keys as the running server
pub async fn initialize_jwks_manager(database: &Database) -> Result<JwksManager, AppError> {
    info!("Initializing JWKS manager for RS256 admin tokens...");
    let mut jwks_manager = JwksManager::new();

    match database.load_rsa_keypairs().await {
        Ok(keypairs) if !keypairs.is_empty() => {
            load_existing_keypairs(&mut jwks_manager, keypairs)?;
        }
        Ok(_) => {
            generate_and_persist_keypair(database, &mut jwks_manager).await?;
        }
        Err(e) => {
            generate_ephemeral_keys(&mut jwks_manager, &e)?;
        }
    }
    info!("JWKS manager initialized");
    Ok(jwks_manager)
}

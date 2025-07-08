// ABOUTME: Integration tests for runtime configuration management
// ABOUTME: Tests runtime config creation, overrides, and session management

use pierre_mcp_server::configuration::profiles::ConfigProfile;
use pierre_mcp_server::configuration::runtime::{ConfigValue, ConfigurationManager, RuntimeConfig};
use uuid::Uuid;

#[test]
fn test_runtime_config_creation() {
    let config = RuntimeConfig::new();
    assert_eq!(config.get_profile(), &ConfigProfile::Default);
    assert!(config.get_session_overrides().is_empty());
    assert!(config.get_value("heart_rate.anaerobic_threshold").is_some());
}

#[test]
fn test_config_value_override() {
    let mut config = RuntimeConfig::new();
    let key = "heart_rate.anaerobic_threshold".to_string();

    // Get base value
    let base_value = config.get_value(&key);
    assert!(base_value.is_some());

    // Set override
    config
        .set_override(key.clone(), ConfigValue::Float(90.0))
        .unwrap();

    // Verify override takes precedence
    if let Some(ConfigValue::Float(value)) = config.get_value(&key) {
        assert!((value - 90.0).abs() < f64::EPSILON);
    } else {
        panic!("Expected float value");
    }
}

#[test]
fn test_module_values() {
    let mut config = RuntimeConfig::new();

    // Add some overrides
    config
        .set_override(
            "heart_rate.custom_threshold".into(),
            ConfigValue::Float(82.5),
        )
        .unwrap();

    let hr_values = config.get_module_values("heart_rate");
    assert!(hr_values.contains_key("heart_rate.anaerobic_threshold"));
    assert!(hr_values.contains_key("heart_rate.custom_threshold"));
}

#[test]
fn test_change_logging() {
    let mut config = RuntimeConfig::new();

    config
        .set_override("test.parameter".into(), ConfigValue::Float(50.0))
        .unwrap();

    let changes = config.get_recent_changes(10);
    assert_eq!(changes.len(), 1);
    assert_eq!(changes[0].parameter, "test.parameter");
}

#[tokio::test]
async fn test_configuration_manager() {
    let manager = ConfigurationManager::new();
    let user_id = Uuid::new_v4();

    // Get config (should create new)
    let config1 = manager.get_user_config(user_id).await;
    assert_eq!(config1.get_profile(), &ConfigProfile::Default);

    // Update config
    manager
        .update_user_config(user_id, |config| {
            config.apply_profile(ConfigProfile::Elite {
                performance_factor: 1.1,
                recovery_sensitivity: 1.2,
            });
            Ok(())
        })
        .await
        .unwrap();

    // Verify update
    let config2 = manager.get_user_config(user_id).await;
    assert!(matches!(config2.get_profile(), ConfigProfile::Elite { .. }));
}

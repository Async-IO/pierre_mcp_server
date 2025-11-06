// ABOUTME: Unit tests for configuration catalog functionality
// ABOUTME: Validates configuration catalog behavior, edge cases, and error handling
//
// Licensed under either of Apache License, Version 2.0 or MIT License at your option.
// Copyright ©2025 Async-IO.org

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#![allow(missing_docs)]

use pierre_mcp_server::configuration::{
    catalog::{CatalogBuilder, ParameterType},
    runtime::ConfigValue,
};

#[test]
fn test_catalog_build() {
    let catalog = CatalogBuilder::build();

    assert!(!catalog.categories.is_empty());
    assert!(catalog.total_parameters > 0);
    assert_eq!(catalog.version, "1.0.0");
}

#[test]
fn test_parameter_lookup() {
    let param = CatalogBuilder::get_parameter("heart_rate.anaerobic_threshold");
    assert!(param.is_some());

    let param = param.unwrap();
    assert_eq!(param.key, "heart_rate.anaerobic_threshold");
    assert!(matches!(param.default_value, ConfigValue::Float(85.0)));
}

#[test]
fn test_module_parameters() {
    let params = CatalogBuilder::get_module_parameters("heart_rate");
    assert!(!params.is_empty());
    assert!(params.iter().all(|p| p.key.starts_with("heart_rate.")));
}

#[test]
fn test_valid_ranges() {
    let catalog = CatalogBuilder::build();

    for category in &catalog.categories {
        for module in &category.modules {
            for param in &module.parameters {
                if let Some(range) = &param.valid_range {
                    match (&param.data_type, range) {
                        (ParameterType::Float, ConfigValue::FloatRange { min, max }) => {
                            assert!(min < max, "Invalid range for {}", param.key);
                        }
                        (ParameterType::Integer, ConfigValue::IntegerRange { min, max }) => {
                            assert!(min < max, "Invalid range for {}", param.key);
                        }
                        _ => panic!("Mismatched range type for {}", param.key),
                    }
                }
            }
        }
    }
}

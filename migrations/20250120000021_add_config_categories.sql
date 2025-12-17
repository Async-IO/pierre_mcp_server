-- ABOUTME: Add new admin configuration categories for providers, caching, MCP, and monitoring
-- ABOUTME: Enables 30 additional configuration parameters to appear in admin UI
--
-- SPDX-License-Identifier: MIT OR Apache-2.0
-- Copyright (c) 2025 Pierre Fitness Intelligence

-- Add missing categories for new parameter definitions
INSERT OR IGNORE INTO admin_config_categories (id, name, display_name, description, display_order, icon) VALUES
    ('cat_cache_ttl', 'cache_ttl', 'Cache TTL', 'Time-to-live settings for various caches', 140, 'clock'),
    ('cat_provider_strava', 'provider_strava', 'Strava Provider', 'Strava API integration settings', 150, 'activity'),
    ('cat_provider_fitbit', 'provider_fitbit', 'Fitbit Provider', 'Fitbit API integration settings', 160, 'watch'),
    ('cat_provider_garmin', 'provider_garmin', 'Garmin Provider', 'Garmin API integration settings', 170, 'watch'),
    ('cat_mcp_network', 'mcp_network', 'MCP Network', 'Model Context Protocol network settings', 180, 'network'),
    ('cat_monitoring', 'monitoring', 'Monitoring', 'System health and performance monitoring settings', 190, 'chart');

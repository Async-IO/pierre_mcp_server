# Fitness Configuration Guide

Pierre MCP Server provides a comprehensive fitness configuration system designed for cloud-native deployment with environment variable-based configuration. This guide covers all fitness-related configuration options for customizing workout analysis, sport type mappings, and intelligence algorithms.

## Overview

Pierre uses a hierarchical configuration system that prioritizes different sources in the following order:

1. **Database** (tenant + user-specific configuration) - highest priority
2. **Database** (tenant default configuration)
3. **Environment variables** (override file/default values) - **primary configuration method**
4. **File configuration** (optional)
5. **Built-in defaults** - lowest priority

## Environment-Based Configuration

All fitness configuration is managed through environment variables defined in `.envrc`. This cloud-native approach ensures easy deployment to any cloud platform without requiring configuration files.

### Sport Type Mappings

Control how external provider sport types are mapped to internal classifications:

```bash
# Sport type mappings (key=value format for env vars)
export SPORT_TYPE_RUN="run"
export SPORT_TYPE_RIDE="bike_ride"
export SPORT_TYPE_SWIM="swim"
export SPORT_TYPE_WALK="walk"
export SPORT_TYPE_HIKE="hike"
export SPORT_TYPE_VIRTUALRIDE="virtual_ride"
export SPORT_TYPE_VIRTUALRUN="virtual_run"
export SPORT_TYPE_WORKOUT="workout"
export SPORT_TYPE_YOGA="yoga"
export SPORT_TYPE_EBIKERIDE="ebike_ride"
export SPORT_TYPE_MOUNTAINBIKERIDE="mountain_bike"
export SPORT_TYPE_GRAVELRIDE="gravel_ride"
```

### Fitness Effort Classification

Configure thresholds for categorizing workout intensity on a 1-10 scale:

```bash
# Fitness Effort Thresholds (1-10 scale)
export FITNESS_EFFORT_LIGHT_MAX="3.0"      # 1.0-3.0 = light effort
export FITNESS_EFFORT_MODERATE_MAX="5.0"   # 3.1-5.0 = moderate effort
export FITNESS_EFFORT_HARD_MAX="7.0"       # 5.1-7.0 = hard effort
# > 7.0 = very_high effort
```

### Heart Rate Zone Configuration

Set heart rate zone thresholds as percentage of maximum heart rate:

```bash
# Heart Rate Zone Thresholds (percentage of max HR)
export FITNESS_ZONE_RECOVERY_MAX="60.0"    # Zone 1: Recovery (up to 60% max HR)
export FITNESS_ZONE_ENDURANCE_MAX="70.0"   # Zone 2: Endurance (60-70% max HR)
export FITNESS_ZONE_TEMPO_MAX="80.0"       # Zone 3: Tempo (70-80% max HR)
export FITNESS_ZONE_THRESHOLD_MAX="90.0"   # Zone 4: Threshold (80-90% max HR)
# > 90.0 = Zone 5: VO2 Max (90%+ max HR)
```

### Weather Integration Settings

Configure weather data integration for activity analysis:

```bash
# Weather Integration
export FITNESS_WEATHER_WIND_THRESHOLD="15.0"           # Wind speed threshold (mph/kph)
export FITNESS_WEATHER_ENABLED="true"                  # Enable weather data integration
export FITNESS_WEATHER_CACHE_DURATION_HOURS="24"      # Cache weather data for 24 hours
export FITNESS_WEATHER_REQUEST_TIMEOUT_SECONDS="10"   # Request timeout
export FITNESS_WEATHER_RATE_LIMIT_PER_MINUTE="60"     # Rate limit for weather API calls
```

### Personal Records Configuration

Set thresholds for detecting personal records and achievements:

```bash
# Personal Records Configuration
export FITNESS_PR_PACE_IMPROVEMENT_THRESHOLD="5.0"    # Pace improvement threshold (%)
```

### Activity Fetch Limits

Control data retrieval limits for performance optimization:

```bash
# Activity Fetch Limits
export MAX_ACTIVITIES_FETCH="100"      # Maximum activities to fetch in one request
export DEFAULT_ACTIVITIES_LIMIT="20"   # Default limit for activity queries
```

## Configuration Structure

The fitness configuration system is implemented in `src/config/fitness_config.rs` and consists of several key components:

### FitnessConfig Structure

```rust
pub struct FitnessConfig {
    pub sport_types: HashMap<String, String>,          // Sport type mappings
    pub intelligence: IntelligenceConfig,               // Analysis parameters
    pub weather_api: Option<WeatherApiConfig>,          // Weather integration
}
```

### Intelligence Configuration

```rust
pub struct IntelligenceConfig {
    pub effort_thresholds: EffortThresholds,           // Workout intensity levels
    pub zone_thresholds: ZoneThresholds,                // Heart rate zones
    pub weather_mapping: WeatherMapping,                // Weather detection
    pub personal_records: PersonalRecordConfig,         // PR detection
}
```

## Supported Sport Types

Pierre supports a comprehensive range of sport types with configurable mappings:

### Standard Activities
- **Run**: Running activities and variations
- **Ride**: Cycling activities (road, mountain, gravel)
- **Swim**: Swimming activities
- **Walk**: Walking and hiking activities

### Virtual/Indoor Activities
- **VirtualRide**: Indoor cycling (Zwift, Peloton, etc.)
- **VirtualRun**: Treadmill running
- **Workout**: General fitness workouts
- **Yoga**: Yoga and stretching sessions

### Specialty Cycling
- **EBikeRide**: Electric bike activities
- **MountainBikeRide**: Mountain biking
- **GravelRide**: Gravel cycling

### Winter Sports
- **CrossCountrySkiing**: Nordic skiing
- **AlpineSkiing**: Downhill skiing
- **Snowboarding**: Snowboard activities
- **Snowshoe**: Snowshoeing
- **IceSkate**: Ice skating
- **BackcountrySki**: Backcountry skiing

### Water Sports
- **Kayaking**: Kayak activities
- **Canoeing**: Canoe activities
- **Rowing**: Rowing activities
- **StandUpPaddling**: SUP/Paddleboarding
- **Surfing**: Surfing activities
- **Kitesurf**: Kitesurfing

### Strength and Fitness
- **WeightTraining**: Strength training
- **Crossfit**: CrossFit workouts
- **Pilates**: Pilates sessions

### Team and Racquet Sports
- **Soccer**: Soccer/Football
- **Basketball**: Basketball
- **Tennis**: Tennis
- **Golf**: Golf activities

## Database-Driven Configuration

For multi-tenant deployments, fitness configuration can be stored in the database with tenant and user-specific overrides.

### Configuration Hierarchy

1. **User-specific configuration**: Custom settings for individual users
2. **Tenant configuration**: Organization-wide settings
3. **Environment variables**: System-wide defaults
4. **Built-in defaults**: Fallback values

### Loading Configuration

```rust
// Load configuration with database support
let config = FitnessConfig::load_for_user(
    Some(&db_manager),          // Database manager
    Some("tenant-id"),          // Tenant ID
    Some("user-id"),            // User ID (optional)
    Some("profile-name")        // Configuration profile name
).await?;
```

### Configuration Tools

Pierre provides MCP tools for managing fitness configuration:

| Tool | Description | Parameters |
|------|-------------|------------|
| `get_fitness_config` | Get current fitness configuration | None |
| `set_fitness_config` | Set fitness configuration parameters | `config` |
| `list_fitness_configs` | List available fitness configurations | None |
| `delete_fitness_config` | Delete a fitness configuration | `config_id` |

## Default Values

Pierre includes scientifically-backed default values for all configuration parameters:

### Effort Thresholds (1-10 scale)
- **Light effort**: 1.0-3.0 (recovery, easy activities)
- **Moderate effort**: 3.1-5.0 (aerobic base training)
- **Hard effort**: 5.1-7.0 (tempo and threshold training)
- **Very high effort**: 7.1-10.0 (VO2 max and anaerobic training)

### Heart Rate Zones (% of max HR)
- **Zone 1 (Recovery)**: Up to 60% - Active recovery
- **Zone 2 (Endurance)**: 60-70% - Aerobic base building
- **Zone 3 (Tempo)**: 70-80% - Aerobic threshold
- **Zone 4 (Threshold)**: 80-90% - Lactate threshold
- **Zone 5 (VO2 Max)**: 90%+ - Neuromuscular power

### Weather Thresholds
- **Wind threshold**: 15.0 mph/kph for significant weather impact
- **Cache duration**: 24 hours for weather data
- **Request timeout**: 10 seconds for weather API calls

## Best Practices

### Environment Variable Management

1. **Use .envrc**: Store all configuration in `.envrc` for development
2. **Production deployment**: Use container environment variables or secrets management
3. **Version control**: Do not commit `.envrc` with real API keys

### Performance Optimization

1. **Cache configuration**: Fitness configuration is loaded once per request
2. **Database caching**: Database-driven configurations are cached in memory
3. **Minimal API calls**: Weather data is cached to reduce external API usage

### Multi-tenant Configuration

1. **Tenant isolation**: Each tenant can have custom fitness parameters
2. **User overrides**: Individual users can customize their fitness settings
3. **Default fallbacks**: System-wide defaults ensure all tenants have working configuration

## Advanced Configuration

### Custom Sport Types

Add support for new sport types by extending the environment variables:

```bash
# Add custom sport type
export SPORT_TYPE_CUSTOMACTIVITY="custom_activity"
```

### Weather Provider Integration

Configure weather data integration with OpenWeather API:

```bash
# Weather Service Configuration
export OPENWEATHER_API_KEY="your_api_key_here"
export FITNESS_WEATHER_ENABLED="true"
export FITNESS_WEATHER_CACHE_DURATION_HOURS="24"
export FITNESS_WEATHER_REQUEST_TIMEOUT_SECONDS="10"
export FITNESS_WEATHER_RATE_LIMIT_PER_MINUTE="60"
```

### Development and Testing

For development environments, you can override specific parameters:

```bash
# Development overrides
export FITNESS_EFFORT_LIGHT_MAX="2.5"      # More sensitive effort detection
export FITNESS_WEATHER_ENABLED="false"     # Disable weather API for testing
export MAX_ACTIVITIES_FETCH="50"           # Smaller fetch limits for testing
```

## Troubleshooting

### Configuration Not Loading

1. **Check environment variables**: Ensure all required variables are set
2. **Verify database connection**: Database-driven configuration requires valid connection
3. **Check logs**: Enable debug logging with `RUST_LOG=debug`

### Invalid Configuration Values

1. **Numeric validation**: Ensure thresholds are valid floating-point numbers
2. **Range validation**: Verify values are within expected ranges (e.g., percentages 0-100)
3. **Boolean values**: Use "true"/"false" for boolean environment variables

### Performance Issues

1. **Reduce fetch limits**: Lower `MAX_ACTIVITIES_FETCH` for slower systems
2. **Disable weather**: Set `FITNESS_WEATHER_ENABLED="false"` to reduce API calls
3. **Increase cache duration**: Higher `FITNESS_WEATHER_CACHE_DURATION_HOURS` reduces API usage

## Related Documentation

- [Plugin System](18-plugin-system.md) - Creating custom fitness analysis plugins
- [Configuration](12-configuration.md) - General server configuration
- [Database](08-database.md) - Database configuration and management
- [API Reference](14-api-reference.md) - Fitness configuration API endpoints
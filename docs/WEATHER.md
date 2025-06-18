# Weather Integration

The server includes comprehensive weather integration that automatically enhances activity analysis with contextual weather data.

## Features

- ✅ **Real-time Weather**: Current weather data from OpenWeatherMap
- ✅ **Historical Weather**: Historical weather data for past activities (with subscription)
- ✅ **GPS-Based**: Extracts coordinates from activity start locations
- ✅ **Smart Fallback**: Intelligent mock weather when API unavailable
- ✅ **Activity Intelligence**: Weather context in activity summaries
- ✅ **Impact Analysis**: Weather difficulty and performance adjustments

## Setup (Optional)

Weather integration works out-of-the-box with realistic mock weather patterns. For real weather data:

1. **Get OpenWeatherMap API Key** (free tier available)
   - Visit https://openweathermap.org/api
   - Sign up for free account
   - Copy your API key

2. **Set Environment Variable**
   ```bash
   export OPENWEATHER_API_KEY="your_api_key_here"
   ```

3. **Configure Settings** (optional)
   Edit `fitness_config.toml`:
   ```toml
   [weather_api]
   provider = "openweathermap"
   enabled = true
   cache_duration_hours = 24
   fallback_to_mock = true
   ```

## Weather Intelligence Examples

With weather integration, activity analysis includes contextual insights:

```json
{
  "summary": "Morning run in the rain with moderate intensity",
  "contextual_factors": {
    "weather": {
      "temperature_celsius": 15.2,
      "humidity_percentage": 85.0,
      "wind_speed_kmh": 12.5,
      "conditions": "rain"
    },
    "time_of_day": "morning"
  }
}
```

## Weather Features

| Feature | Free Tier | Paid Tier |
|---------|-----------|-----------|
| **Mock Weather** | ✅ Realistic patterns | ✅ Available |
| **Current Weather** | ✅ Real-time data | ✅ Real-time data |
| **Historical Weather** | 🎭 Mock fallback | ✅ Real historical data |
| **API Calls** | 1,000/day free | Unlimited with subscription |
| **Production Ready** | ✅ Zero costs | ✅ Precise data |

## Testing Weather Integration

```bash
# Test weather system
cargo run --bin test-weather-integration

# Diagnose API setup
cargo run --bin diagnose-weather-api
```

## API Integration Details

### OpenWeatherMap Configuration

The weather service integrates with OpenWeatherMap's API:

- **Current Weather**: Uses the Current Weather Data API
- **Historical Weather**: Uses the Historical Weather Data API (requires subscription)
- **Geocoding**: Uses GPS coordinates from activities to fetch location-specific weather
- **Caching**: Weather data is cached to minimize API calls and improve performance

### Mock Weather System

When the OpenWeatherMap API is unavailable or not configured, Pierre uses an intelligent mock weather system:

- **Realistic Patterns**: Generates weather patterns based on location and season
- **Consistency**: Weather data remains consistent for the same date/location combinations
- **Variety**: Includes various weather conditions (sunny, rainy, cloudy, windy)
- **Temperature Modeling**: Realistic temperature ranges based on geography and season

### Weather Context in Activity Intelligence

Weather data enhances activity analysis by providing:

1. **Performance Context**: Understanding how weather conditions affected performance
2. **Difficulty Assessment**: Adjusting perceived effort based on environmental challenges
3. **Training Insights**: Identifying patterns in performance under different conditions
4. **Safety Considerations**: Highlighting potentially dangerous weather conditions

### Example Weather-Enhanced Analysis

```json
{
  "activity_intelligence": {
    "summary": "Challenging trail run in adverse weather conditions",
    "weather_impact": {
      "conditions": "Heavy rain, 8°C, 25 km/h winds",
      "difficulty_multiplier": 1.4,
      "performance_adjustment": "Performance 15% above expected given conditions",
      "insights": [
        "Excellent pace maintenance despite heavy rain",
        "Strong performance in challenging wind conditions",
        "Temperature optimal for sustained effort"
      ]
    },
    "recommendations": [
      "Consider similar weather training for race preparation",
      "Excellent mental toughness demonstrated",
      "Gear performed well in wet conditions"
    ]
  }
}
```
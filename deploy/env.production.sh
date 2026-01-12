# ABOUTME: Production environment configuration for GCP deployment
# ABOUTME: Mirrors .envrc structure but with GCP-appropriate values

# =============================================================================
# BACKEND CONFIGURATION (Cloud Run environment variables)
# =============================================================================
# Note: Secrets (marked [SECRET]) are stored in Secret Manager, not here

# Server Configuration
export RUST_LOG="info"
export HTTP_PORT="8081"

# Database - Cloud SQL connection via Unix socket
# DATABASE_URL is constructed in deploy.sh using Cloud SQL socket path
export POSTGRES_MAX_CONNECTIONS="10"
export POSTGRES_MIN_CONNECTIONS="2"
export POSTGRES_ACQUIRE_TIMEOUT="30"

# Security
export PIERRE_RSA_KEY_SIZE="4096"
export JWT_EXPIRY_HOURS="24"
# [SECRET] PIERRE_MASTER_ENCRYPTION_KEY - stored in Secret Manager

# OAuth Providers - Client IDs are not secret, secrets are in Secret Manager
export STRAVA_CLIENT_ID=""           # Set your client ID here
export FITBIT_CLIENT_ID=""           # Set your client ID here
export GARMIN_CLIENT_ID=""           # Set your client ID here
export COROS_CLIENT_ID=""            # Set your client ID here
# [SECRET] *_CLIENT_SECRET - stored in Secret Manager
# Redirect URIs are set dynamically based on Cloud Run URL

# Firebase (Google Sign-In backend validation)
export FIREBASE_PROJECT_ID="pierre-fitness-intelligence"

# LLM Provider Configuration
# Options: groq, gemini, vertex, local
# - groq: Fast inference via Groq cloud (requires GROQ_API_KEY secret)
# - gemini: Google AI Studio API (requires GEMINI_API_KEY secret, has rate limits)
# - vertex: Google Vertex AI (recommended for GCP - uses service account auth, no API key needed)
# - local: Local Ollama or OpenAI-compatible server
export PIERRE_LLM_PROVIDER="vertex"

# GCP Configuration (required for Vertex AI provider)
# GCP_PROJECT_ID is sourced from config.sh
# export GCP_PROJECT_ID="pierrefitnessplatform"  # Set in config.sh
# export GCP_REGION="northamerica-northeast1"     # Set in config.sh

# [SECRET] GROQ_API_KEY - stored in Secret Manager (if using Groq)
# [SECRET] GEMINI_API_KEY - stored in Secret Manager (if using Gemini AI Studio)

# Weather Service
# [SECRET] OPENWEATHER_API_KEY - stored in Secret Manager

# USDA Nutrition API
# [SECRET] USDA_API_KEY - stored in Secret Manager

# Cache Configuration (in-memory for now, Redis later)
export CACHE_MAX_ENTRIES="10000"
export CACHE_CLEANUP_INTERVAL_SECS="300"
# export REDIS_URL=""  # Uncomment when adding Memorystore

# Rate Limiting
export RATE_LIMIT_ENABLED="true"
export RATE_LIMIT_REQUESTS="100"
export RATE_LIMIT_WINDOW="60"

# Activity Limits
export MAX_ACTIVITIES_FETCH="100"
export DEFAULT_ACTIVITIES_LIMIT="20"

# Fitness Configuration
export FITNESS_EFFORT_LIGHT_MAX="3.0"
export FITNESS_EFFORT_MODERATE_MAX="5.0"
export FITNESS_EFFORT_HARD_MAX="7.0"
export FITNESS_ZONE_RECOVERY_MAX="60.0"
export FITNESS_ZONE_ENDURANCE_MAX="70.0"
export FITNESS_ZONE_TEMPO_MAX="80.0"
export FITNESS_ZONE_THRESHOLD_MAX="90.0"
export FITNESS_WEATHER_ENABLED="true"
export FITNESS_WEATHER_CACHE_DURATION_HOURS="24"
export FITNESS_WEATHER_REQUEST_TIMEOUT_SECONDS="10"
export FITNESS_WEATHER_RATE_LIMIT_PER_MINUTE="60"
export FITNESS_WEATHER_WIND_THRESHOLD="15.0"
export FITNESS_PR_PACE_IMPROVEMENT_THRESHOLD="5.0"

# =============================================================================
# FRONTEND CONFIGURATION (Build-time Vite environment variables)
# =============================================================================
# These are baked into the frontend at build time

# API Base URL - will be Cloud Run service URL
export VITE_API_BASE_URL=""  # Set after first deployment

# Firebase Configuration (Google Sign-In)
export VITE_FIREBASE_API_KEY="AIzaSyAYYmGwtoZK1xWdZqkrKHQTgsw6I3ExZjY"
export VITE_FIREBASE_AUTH_DOMAIN="pierre-fitness-intelligence.firebaseapp.com"
export VITE_FIREBASE_PROJECT_ID="pierre-fitness-intelligence"
export VITE_FIREBASE_STORAGE_BUCKET="pierre-fitness-intelligence.firebasestorage.app"
export VITE_FIREBASE_MESSAGING_SENDER_ID="779931405774"
export VITE_FIREBASE_APP_ID="1:779931405774:web:949695e2beb6e3f5da6f9f"
export VITE_FIREBASE_MEASUREMENT_ID="G-BQX1HG5J0Y"

# =============================================================================
# SDK CONFIGURATION
# =============================================================================
# SDK just needs server URL, which will be the Cloud Run URL
export PIERRE_SERVER_URL=""  # Set after first deployment

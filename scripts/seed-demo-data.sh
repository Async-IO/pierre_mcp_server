#!/bin/bash
# ABOUTME: Database seed script for Pierre dashboard demo data
# ABOUTME: Populates users, API keys, usage statistics, and A2A clients

set -e

# Configuration
DB_PATH="${DATABASE_URL:-./data/users.db}"
# Strip sqlite: prefix if present
DB_PATH="${DB_PATH#sqlite:}"

echo "=== Pierre Demo Data Seeder ==="
echo "Database: $DB_PATH"

# Check if database exists
if [ ! -f "$DB_PATH" ]; then
    echo "Error: Database not found at $DB_PATH"
    echo "Run the server first to create the database with migrations."
    exit 1
fi

# Generate UUIDs (macOS compatible)
generate_uuid() {
    uuidgen | tr '[:upper:]' '[:lower:]'
}

# Generate ISO timestamp
now_iso() {
    date -u +"%Y-%m-%dT%H:%M:%SZ"
}

# Generate timestamp N days ago
days_ago_iso() {
    local days=$1
    if [[ "$OSTYPE" == "darwin"* ]]; then
        date -u -v-${days}d +"%Y-%m-%dT%H:%M:%SZ"
    else
        date -u -d "$days days ago" +"%Y-%m-%dT%H:%M:%SZ"
    fi
}

# Generate random timestamp within last N days
random_timestamp_within_days() {
    local max_days=$1
    local random_days=$((RANDOM % max_days))
    local random_hours=$((RANDOM % 24))
    local random_mins=$((RANDOM % 60))
    if [[ "$OSTYPE" == "darwin"* ]]; then
        date -u -v-${random_days}d -v-${random_hours}H -v-${random_mins}M +"%Y-%m-%dT%H:%M:%SZ"
    else
        date -u -d "$random_days days ago $random_hours hours ago $random_mins minutes ago" +"%Y-%m-%dT%H:%M:%SZ"
    fi
}

# Hash password using SHA256 (demo purposes - real passwords use bcrypt)
hash_password() {
    echo -n "$1" | openssl dgst -sha256 | awk '{print $2}'
}

echo ""
echo "Step 1: Creating demo users..."

# Get existing admin user ID if any
ADMIN_USER_ID=$(sqlite3 "$DB_PATH" "SELECT id FROM users WHERE is_admin = 1 LIMIT 1;" 2>/dev/null || echo "")

if [ -z "$ADMIN_USER_ID" ]; then
    echo "  No admin user found. Creating admin user..."
    ADMIN_USER_ID=$(generate_uuid)
    ADMIN_PWD_HASH='$2b$12$demo.admin.password.hash.placeholder'
    sqlite3 "$DB_PATH" "INSERT INTO users (id, email, display_name, password_hash, tier, is_active, user_status, is_admin, created_at, last_active) VALUES ('$ADMIN_USER_ID', 'admin@pierre.dev', 'Pierre Admin', '$ADMIN_PWD_HASH', 'enterprise', 1, 'active', 1, '$(now_iso)', '$(now_iso)');"
    echo "  Created admin user: admin@pierre.dev"
else
    echo "  Using existing admin user: $ADMIN_USER_ID"
fi

# Create demo users with various statuses
# Note: users table only allows starter/professional/enterprise tiers (no trial)
USERS=(
    "alice@acme.com|Alice Johnson|professional|active"
    "bob@startup.io|Bob Smith|starter|active"
    "charlie@enterprise.co|Charlie Brown|enterprise|active"
    "diana@freelance.dev|Diana Prince|starter|active"
    "eve@pending.com|Eve Wilson|starter|pending"
    "frank@pending.org|Frank Miller|starter|pending"
    "grace@suspended.net|Grace Lee|professional|suspended"
    "henry@techcorp.io|Henry Zhang|enterprise|active"
    "isabella@fitness.app|Isabella Martinez|professional|active"
    "james@healthtrack.com|James OBrien|starter|active"
    "kate@runclub.org|Kate Williams|starter|active"
    "leo@gym.pro|Leo Thompson|professional|active"
    "maya@cycling.io|Maya Patel|enterprise|active"
    "nathan@swim.club|Nathan Kim|starter|pending"
    "olivia@triathlon.net|Olivia Chen|professional|active"
    "peter@crossfit.gym|Peter Anderson|starter|active"
    "quinn@yoga.space|Quinn Murphy|starter|pending"
    "rachel@pilates.studio|Rachel Green|professional|suspended"
)

declare -a USER_IDS
USER_IDS+=("$ADMIN_USER_ID")

for user_data in "${USERS[@]}"; do
    IFS='|' read -r email name tier status <<< "$user_data"

    # Check if user already exists
    EXISTING=$(sqlite3 "$DB_PATH" "SELECT id FROM users WHERE email = '$email';" 2>/dev/null || echo "")

    if [ -z "$EXISTING" ]; then
        USER_ID=$(generate_uuid)
        PWD_HASH='$2b$12$demo.password.hash.placeholder.here'
        CREATED_AT=$(random_timestamp_within_days 60)
        IS_ACTIVE=$([[ "$status" == "suspended" ]] && echo "0" || echo "1")
        IS_ADMIN=0

        sqlite3 "$DB_PATH" "INSERT INTO users (id, email, display_name, password_hash, tier, is_active, user_status, is_admin, created_at, last_active) VALUES ('$USER_ID', '$email', '$name', '$PWD_HASH', '$tier', $IS_ACTIVE, '$status', $IS_ADMIN, '$CREATED_AT', '$(now_iso)');"
        echo "  Created user: $email ($status, $tier)"
        USER_IDS+=("$USER_ID")
    else
        echo "  Skipping existing user: $email"
        USER_IDS+=("$EXISTING")
    fi
done

echo ""
echo "Step 2: Creating API keys..."

# API key names and configs
API_KEYS=(
    "Production API|Main production workload|professional|10000|3600"
    "Staging Environment|Pre-production testing|starter|1000|3600"
    "Mobile App Backend|iOS and Android API|professional|5000|3600"
    "Analytics Pipeline|Data processing jobs|enterprise|0|3600"
    "Trial Key - Evaluation|Testing the platform|trial|100|3600"
    "Partner Integration|Third-party integration|starter|2000|3600"
    "Development|Local dev testing|trial|500|3600"
    "High Volume Batch|Batch processing jobs|enterprise|0|3600"
    "Strava Sync|Automated Strava activity sync|professional|3000|3600"
    "Garmin Connect|Garmin device integration|professional|3000|3600"
    "Wahoo Bridge|Wahoo workout imports|starter|1500|3600"
    "Apple Health|HealthKit data sync|professional|5000|3600"
    "Workout Analyzer|AI-powered workout analysis|enterprise|0|3600"
    "Recovery Tracker|Sleep and recovery metrics|starter|1000|3600"
    "Nutrition Logger|Meal and calorie tracking|starter|800|3600"
    "Training Plan Bot|Automated plan generation|professional|4000|3600"
    "Race Predictor|Performance prediction engine|enterprise|0|3600"
    "Social Feed|Activity sharing and comments|starter|2000|3600"
    "Coaching Dashboard|Personal trainer tools|professional|6000|3600"
    "Challenge Manager|Competition and challenge API|starter|1500|3600"
)

declare -a API_KEY_IDS

for i in "${!API_KEYS[@]}"; do
    IFS='|' read -r name desc tier limit window <<< "${API_KEYS[$i]}"

    # Check if API key already exists
    EXISTING=$(sqlite3 "$DB_PATH" "SELECT id FROM api_keys WHERE name = '$name';" 2>/dev/null || echo "")

    if [ -z "$EXISTING" ]; then
        KEY_ID=$(generate_uuid)
        # Assign to different users
        USER_INDEX=$((i % ${#USER_IDS[@]}))
        USER_ID="${USER_IDS[$USER_INDEX]}"

        KEY_PREFIX="pk_$(openssl rand -hex 4)"
        KEY_HASH=$(echo -n "sk_$(openssl rand -hex 16)" | openssl dgst -sha256 | awk '{print $2}')
        CREATED_AT=$(random_timestamp_within_days 30)

        # Set expiry for trial keys (14 days from creation)
        if [ "$tier" == "trial" ]; then
            if [[ "$OSTYPE" == "darwin"* ]]; then
                EXPIRES_AT=$(date -u -v+14d +"%Y-%m-%dT%H:%M:%SZ")
            else
                EXPIRES_AT=$(date -u -d "+14 days" +"%Y-%m-%dT%H:%M:%SZ")
            fi
            EXPIRES_SQL="'$EXPIRES_AT'"
        else
            EXPIRES_SQL="NULL"
        fi

        LIMIT_SQL=$([[ "$limit" == "0" ]] && echo "NULL" || echo "$limit")

        sqlite3 "$DB_PATH" "INSERT INTO api_keys (id, user_id, name, description, key_hash, key_prefix, tier, rate_limit_requests, rate_limit_window_seconds, is_active, expires_at, created_at) VALUES ('$KEY_ID', '$USER_ID', '$name', '$desc', '$KEY_HASH', '$KEY_PREFIX', '$tier', $LIMIT_SQL, $window, 1, $EXPIRES_SQL, '$CREATED_AT');"
        echo "  Created API key: $name ($tier)"
        API_KEY_IDS+=("$KEY_ID")
    else
        echo "  Skipping existing API key: $name"
        API_KEY_IDS+=("$EXISTING")
    fi
done

echo ""
echo "Step 3: Generating API usage data (last 30 days)..."

TOOLS=("get_activities" "analyze_workout" "get_profile" "sync_data" "generate_insights" "get_goals" "update_preferences" "get_recommendations" "get_heart_rate" "get_power_zones" "calculate_ftp" "predict_race" "get_training_load" "analyze_sleep" "get_nutrition_log" "sync_garmin" "sync_strava" "export_gpx" "import_tcx" "get_leaderboard")
STATUS_CODES=(200 200 200 200 200 200 200 200 200 200 201 201 400 401 403 429 500 502 503)

# Generate usage for each API key
for key_id in "${API_KEY_IDS[@]}"; do
    # Get the tier for this key to determine usage volume
    TIER=$(sqlite3 "$DB_PATH" "SELECT tier FROM api_keys WHERE id = '$key_id';" 2>/dev/null || echo "trial")

    case "$TIER" in
        enterprise) BASE_REQUESTS=250 ;;
        professional) BASE_REQUESTS=150 ;;
        starter) BASE_REQUESTS=80 ;;
        *) BASE_REQUESTS=30 ;;
    esac

    # Generate usage for last 30 days
    for day in $(seq 0 29); do
        # Weekend has less traffic
        DAY_OF_WEEK=$(date -v-${day}d +%u 2>/dev/null || date -d "$day days ago" +%u)
        if [ "$DAY_OF_WEEK" -ge 6 ]; then
            DAILY_REQUESTS=$((BASE_REQUESTS / 3 + RANDOM % (BASE_REQUESTS / 4)))
        else
            DAILY_REQUESTS=$((BASE_REQUESTS + RANDOM % (BASE_REQUESTS / 2)))
        fi

        for req in $(seq 1 $DAILY_REQUESTS); do
            USAGE_ID=$(generate_uuid)
            TOOL="${TOOLS[$((RANDOM % ${#TOOLS[@]}))]}"
            STATUS="${STATUS_CODES[$((RANDOM % ${#STATUS_CODES[@]}))]}"
            RESPONSE_TIME=$((50 + RANDOM % 450))

            # Generate timestamp for this day with random hour/minute
            HOUR=$((8 + RANDOM % 12))  # Business hours bias
            MIN=$((RANDOM % 60))
            if [[ "$OSTYPE" == "darwin"* ]]; then
                TIMESTAMP=$(date -u -v-${day}d -v${HOUR}H -v${MIN}M +"%Y-%m-%dT%H:%M:%SZ")
            else
                TIMESTAMP=$(date -u -d "$day days ago $HOUR hours $MIN minutes" +"%Y-%m-%dT%H:%M:%SZ")
            fi

            sqlite3 "$DB_PATH" "INSERT INTO api_key_usage (id, api_key_id, timestamp, tool_name, status_code, response_time_ms) VALUES ('$USAGE_ID', '$key_id', '$TIMESTAMP', '$TOOL', $STATUS, $RESPONSE_TIME);" 2>/dev/null || true
        done
    done
    echo "  Generated usage for key: $key_id"
done

echo ""
echo "Step 4: Creating A2A clients..."

A2A_CLIENTS=(
    "Claude Desktop|AI Assistant Integration|[\"chat\", \"analyze\"]"
    "Fitness Bot|Automated workout analysis|[\"sync\", \"analyze\", \"recommend\"]"
    "Data Pipeline|ETL processing agent|[\"sync\", \"export\"]"
    "GPT-4 Fitness Coach|OpenAI-powered coaching|[\"chat\", \"recommend\", \"plan\"]"
    "Gemini Analyzer|Google AI workout insights|[\"analyze\", \"summarize\"]"
    "Slack Bot|Team fitness notifications|[\"notify\", \"report\"]"
    "Discord Bot|Community challenges|[\"notify\", \"leaderboard\"]"
    "Zapier Integration|Workflow automation|[\"sync\", \"export\", \"webhook\"]"
    "Training Peaks Sync|TrainingPeaks data bridge|[\"sync\", \"import\", \"export\"]"
    "Garmin Agent|Garmin Connect automation|[\"sync\", \"analyze\"]"
)

declare -a A2A_CLIENT_IDS

for i in "${!A2A_CLIENTS[@]}"; do
    IFS='|' read -r name desc capabilities <<< "${A2A_CLIENTS[$i]}"

    EXISTING=$(sqlite3 "$DB_PATH" "SELECT id FROM a2a_clients WHERE name = '$name';" 2>/dev/null || echo "")

    if [ -z "$EXISTING" ]; then
        CLIENT_ID=$(generate_uuid)
        USER_ID="${USER_IDS[$((i % ${#USER_IDS[@]}))]}"
        PUBLIC_KEY="pk_a2a_$(openssl rand -hex 8)"
        CLIENT_SECRET=$(openssl rand -hex 32)
        PERMISSIONS='["read", "write"]'
        CREATED_AT=$(random_timestamp_within_days 45)

        sqlite3 "$DB_PATH" "INSERT INTO a2a_clients (id, user_id, name, description, public_key, client_secret, permissions, capabilities, rate_limit_requests, rate_limit_window_seconds, is_active, created_at, updated_at) VALUES ('$CLIENT_ID', '$USER_ID', '$name', '$desc', '$PUBLIC_KEY', '$CLIENT_SECRET', '$PERMISSIONS', '$capabilities', 1000, 3600, 1, '$CREATED_AT', '$(now_iso)');"
        echo "  Created A2A client: $name"
        A2A_CLIENT_IDS+=("$CLIENT_ID")
    else
        echo "  Skipping existing A2A client: $name"
        A2A_CLIENT_IDS+=("$EXISTING")
    fi
done

echo ""
echo "Step 5: Generating A2A usage data..."

A2A_TOOLS=("send_message" "analyze_activity" "get_recommendations" "sync_data" "export_report")

for client_id in "${A2A_CLIENT_IDS[@]}"; do
    # Generate A2A usage for last 14 days
    for day in $(seq 0 13); do
        DAILY_REQUESTS=$((20 + RANDOM % 30))

        for req in $(seq 1 $DAILY_REQUESTS); do
            USAGE_ID=$(generate_uuid)
            TOOL="${A2A_TOOLS[$((RANDOM % ${#A2A_TOOLS[@]}))]}"
            STATUS="${STATUS_CODES[$((RANDOM % ${#STATUS_CODES[@]}))]}"
            RESPONSE_TIME=$((100 + RANDOM % 500))

            HOUR=$((RANDOM % 24))
            MIN=$((RANDOM % 60))
            if [[ "$OSTYPE" == "darwin"* ]]; then
                TIMESTAMP=$(date -u -v-${day}d -v${HOUR}H -v${MIN}M +"%Y-%m-%dT%H:%M:%SZ")
            else
                TIMESTAMP=$(date -u -d "$day days ago $HOUR hours $MIN minutes" +"%Y-%m-%dT%H:%M:%SZ")
            fi

            sqlite3 "$DB_PATH" "INSERT INTO a2a_usage (id, client_id, timestamp, tool_name, status_code, response_time_ms, protocol_version) VALUES ('$USAGE_ID', '$client_id', '$TIMESTAMP', '$TOOL', $STATUS, $RESPONSE_TIME, '1.0');" 2>/dev/null || true
        done
    done
    echo "  Generated A2A usage for client: $client_id"
done

echo ""
echo "Step 6: Creating admin service tokens..."

ADMIN_TOKENS=(
    "CI/CD Pipeline|GitHub Actions deployment automation"
    "API Gateway|Production load balancer service"
    "Monitoring Service|Health check and alerting system"
    "Backup Automation|Database backup and restore jobs"
    "Key Rotation Bot|Automated credential rotation"
    "User Provisioner|Bulk user onboarding service"
    "Analytics Collector|Usage metrics aggregation"
    "Security Scanner|Vulnerability assessment tool"
    "Audit Logger|Compliance audit trail service"
    "Support Portal|Customer support key management"
)

for i in "${!ADMIN_TOKENS[@]}"; do
    IFS='|' read -r service_name service_desc <<< "${ADMIN_TOKENS[$i]}"

    EXISTING=$(sqlite3 "$DB_PATH" "SELECT id FROM admin_tokens WHERE service_name = '$service_name';" 2>/dev/null || echo "")

    if [ -z "$EXISTING" ]; then
        TOKEN_ID=$(generate_uuid)
        TOKEN_PREFIX="ADMIN-$(openssl rand -hex 4 | tr '[:lower:]' '[:upper:]')"
        TOKEN_HASH=$(openssl rand -hex 32)
        JWT_SECRET_HASH=$(openssl rand -hex 32)
        PERMISSIONS='["provision_keys", "revoke_keys", "view_analytics"]'
        CREATED_AT=$(random_timestamp_within_days 60)

        sqlite3 "$DB_PATH" "INSERT INTO admin_tokens (id, service_name, service_description, token_hash, token_prefix, jwt_secret_hash, permissions, is_super_admin, is_active, created_at, usage_count) VALUES ('$TOKEN_ID', '$service_name', '$service_desc', '$TOKEN_HASH', '$TOKEN_PREFIX', '$JWT_SECRET_HASH', '$PERMISSIONS', 0, 1, '$CREATED_AT', $((RANDOM % 100)));"
        echo "  Created admin token: $service_name ($TOKEN_PREFIX)"
    else
        echo "  Skipping existing admin token: $service_name"
    fi
done

echo ""
echo "Step 7: Generating request logs..."

ENDPOINTS=("/api/activities" "/api/profile" "/api/analytics" "/api/sync" "/api/goals" "/api/insights" "/api/workouts" "/api/heart-rate" "/api/power" "/api/sleep" "/api/nutrition" "/api/training-load" "/api/recommendations" "/api/export" "/api/import")
METHODS=("GET" "GET" "GET" "POST" "GET" "GET" "GET" "GET" "GET" "GET" "POST" "GET" "GET" "POST" "POST")

# Generate request logs for the last 14 days
for day in $(seq 0 13); do
    DAILY_LOGS=$((200 + RANDOM % 250))

    for log in $(seq 1 $DAILY_LOGS); do
        LOG_ID=$(generate_uuid)
        USER_ID="${USER_IDS[$((RANDOM % ${#USER_IDS[@]}))]}"
        ENDPOINT_IDX=$((RANDOM % ${#ENDPOINTS[@]}))
        ENDPOINT="${ENDPOINTS[$ENDPOINT_IDX]}"
        METHOD="${METHODS[$ENDPOINT_IDX]}"
        STATUS="${STATUS_CODES[$((RANDOM % ${#STATUS_CODES[@]}))]}"
        RESPONSE_TIME=$((30 + RANDOM % 300))

        HOUR=$((RANDOM % 24))
        MIN=$((RANDOM % 60))
        if [[ "$OSTYPE" == "darwin"* ]]; then
            TIMESTAMP=$(date -u -v-${day}d -v${HOUR}H -v${MIN}M +"%Y-%m-%dT%H:%M:%SZ")
        else
            TIMESTAMP=$(date -u -d "$day days ago $HOUR hours $MIN minutes" +"%Y-%m-%dT%H:%M:%SZ")
        fi

        sqlite3 "$DB_PATH" "INSERT INTO request_logs (id, user_id, timestamp, method, endpoint, status_code, response_time_ms) VALUES ('$LOG_ID', '$USER_ID', '$TIMESTAMP', '$METHOD', '$ENDPOINT', $STATUS, $RESPONSE_TIME);" 2>/dev/null || true
    done
done
echo "  Generated request logs for last 14 days"

echo ""
echo "=== Summary ==="
echo "Users created: $(sqlite3 "$DB_PATH" "SELECT COUNT(*) FROM users;")"
echo "API keys created: $(sqlite3 "$DB_PATH" "SELECT COUNT(*) FROM api_keys;")"
echo "API usage records: $(sqlite3 "$DB_PATH" "SELECT COUNT(*) FROM api_key_usage;")"
echo "A2A clients created: $(sqlite3 "$DB_PATH" "SELECT COUNT(*) FROM a2a_clients;")"
echo "A2A usage records: $(sqlite3 "$DB_PATH" "SELECT COUNT(*) FROM a2a_usage;")"
echo "Admin tokens created: $(sqlite3 "$DB_PATH" "SELECT COUNT(*) FROM admin_tokens;")"
echo "Request logs created: $(sqlite3 "$DB_PATH" "SELECT COUNT(*) FROM request_logs;")"
echo "Pending users: $(sqlite3 "$DB_PATH" "SELECT COUNT(*) FROM users WHERE user_status = 'pending';")"
echo ""
echo "Done! Restart the server to see the demo data in the dashboard."

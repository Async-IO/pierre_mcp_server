#!/bin/bash
# Helper script to run Docker Compose with .envrc environment variables
set -e

echo "Loading environment variables from .envrc..."

# Check if .envrc exists
if [ ! -f ".envrc" ]; then
    echo "Error: .envrc file not found. Please create it from .env.example"
    exit 1
fi

# Source .envrc variables by parsing export statements
while IFS= read -r line; do
    if [[ $line =~ ^export[[:space:]]+([^=]+)=(.+)$ ]]; then
        var_name="${BASH_REMATCH[1]}"
        var_value="${BASH_REMATCH[2]}"
        # Remove quotes if present
        var_value=$(echo "$var_value" | sed 's/^"//;s/"$//')
        export "$var_name"="$var_value"
        echo "Loaded $var_name"
    fi
done < .envrc

echo "Starting Docker Compose with loaded environment variables..."

# Run docker-compose with all arguments passed to this script
exec docker-compose "$@"
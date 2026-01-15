#!/usr/bin/env bash
# ABOUTME: Manage secrets in GCP Secret Manager
# ABOUTME: Add, update, list, and configure OAuth provider secrets

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "${SCRIPT_DIR}/../config.sh"

usage() {
    cat << EOF
Usage: $0 <command> [args]

Commands:
    list                      List all Pierre secrets
    add <name> <value>        Create or update a secret
    get <name>                Get the latest version of a secret
    delete <name>             Delete a secret

    setup-oauth               Interactive setup for all OAuth providers
    add-oauth <provider>      Add OAuth credentials for a provider
                              (strava, fitbit, garmin, coros)

Examples:
    $0 list
    $0 add strava-client-id "12345"
    $0 add strava-client-secret "abc123"
    $0 setup-oauth
    $0 add-oauth strava
EOF
    exit 1
}

prefix_name() {
    echo "${SERVICE_NAME}-$1"
}

cmd_list() {
    echo "=== Pierre Secrets ==="
    gcloud secrets list --filter="labels.app=pierre" --format="table(name,createTime)"
}

cmd_add() {
    local name="$1"
    local value="$2"
    local secret_name
    secret_name=$(prefix_name "${name}")

    echo ">>> Adding secret: ${secret_name}"

    if gcloud secrets describe "${secret_name}" &>/dev/null; then
        echo "Secret exists. Adding new version..."
        echo -n "${value}" | gcloud secrets versions add "${secret_name}" --data-file=-
    else
        echo "Creating new secret..."
        echo -n "${value}" | gcloud secrets create "${secret_name}" \
            --replication-policy="automatic" \
            --labels="app=pierre" \
            --data-file=-
    fi
    echo "Done."
}

cmd_get() {
    local name="$1"
    local secret_name
    secret_name=$(prefix_name "${name}")

    gcloud secrets versions access latest --secret="${secret_name}"
}

cmd_delete() {
    local name="$1"
    local secret_name
    secret_name=$(prefix_name "${name}")

    echo ">>> Deleting secret: ${secret_name}"
    gcloud secrets delete "${secret_name}" --quiet
    echo "Done."
}

cmd_add_oauth() {
    local provider="$1"
    provider=$(echo "${provider}" | tr '[:upper:]' '[:lower:]')

    echo "=== Add OAuth credentials for ${provider} ==="

    read -rp "Client ID: " client_id
    read -rsp "Client Secret: " client_secret
    echo ""
    read -rp "Redirect URI (or press Enter for default): " redirect_uri

    cmd_add "${provider}-client-id" "${client_id}"
    cmd_add "${provider}-client-secret" "${client_secret}"

    if [[ -n "${redirect_uri}" ]]; then
        cmd_add "${provider}-redirect-uri" "${redirect_uri}"
    fi

    echo ""
    echo "OAuth credentials for ${provider} configured."
}

cmd_setup_oauth() {
    echo "=== OAuth Provider Setup ==="
    echo "Configure credentials for fitness providers."
    echo ""

    for provider in strava fitbit garmin coros; do
        read -rp "Configure ${provider}? (y/N): " yn
        if [[ "${yn}" =~ ^[Yy]$ ]]; then
            cmd_add_oauth "${provider}"
            echo ""
        fi
    done

    echo "=== OAuth Setup Complete ==="
    echo ""
    echo "To use these secrets in Cloud Run, redeploy with:"
    echo "  ./scripts/deploy.sh"
}

# Main
[[ $# -lt 1 ]] && usage

case "$1" in
    list)
        cmd_list
        ;;
    add)
        [[ $# -lt 3 ]] && usage
        cmd_add "$2" "$3"
        ;;
    get)
        [[ $# -lt 2 ]] && usage
        cmd_get "$2"
        ;;
    delete)
        [[ $# -lt 2 ]] && usage
        cmd_delete "$2"
        ;;
    add-oauth)
        [[ $# -lt 2 ]] && usage
        cmd_add_oauth "$2"
        ;;
    setup-oauth)
        cmd_setup_oauth
        ;;
    *)
        usage
        ;;
esac

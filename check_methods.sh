#!/bin/bash

# List of methods to check
methods=(
    "get_users_by_status_cursor"
    "update_user_status"
    "get_user_insights"
    "get_api_keys_filtered"
    "get_api_key_usage_stats"
    "create_a2a_client"
    "get_a2a_client_credentials"
    "create_a2a_session"
    "create_a2a_task"
    "list_a2a_tasks"
    "update_a2a_task_status"
    "get_a2a_usage_stats"
    "get_a2a_client_usage_history"
    "get_provider_last_sync"
    "update_provider_last_sync"
    "get_top_tools_analysis"
    "create_admin_token"
    "get_admin_token_by_id"
    "get_admin_token_by_prefix"
    "list_admin_tokens"
    "update_admin_token_last_used"
    "record_admin_token_usage"
    "get_admin_token_usage_history"
    "record_admin_provisioned_key"
    "get_admin_provisioned_keys"
    "save_rsa_keypair"
    "load_rsa_keypairs"
    "list_oauth_apps_for_user"
    "get_refresh_token_by_value"
    "store_authorization_code"
    "store_key_version"
    "get_key_versions"
    "get_current_key_version"
    "update_key_version_status"
    "delete_old_key_versions"
    "get_audit_events"
    "store_oauth_notification"
    "get_unread_oauth_notifications"
    "mark_oauth_notification_read"
    "get_all_oauth_notifications"
    "upsert_user_oauth_token"
    "get_user_oauth_token"
    "get_tenant_provider_tokens"
    "delete_user_oauth_token"
    "refresh_user_oauth_token"
    "store_user_oauth_app"
    "get_user_oauth_app"
)

echo "Checking for inherent implementations in Database struct (lines 53-2258)..."
echo ""

missing_impls=()

for method in "${methods[@]}"; do
    # Check if method exists in inherent impl (between lines 53 and 2258)
    if awk 'NR >= 53 && NR <= 2258' /home/user/pierre_mcp_server/src/database/mod.rs | grep -q "pub async fn $method"; then
        echo "✓ $method - has inherent impl"
    else
        # Check in other database module files
        if find /home/user/pierre_mcp_server/src/database -name "*.rs" ! -name "mod.rs" -exec grep -q "pub async fn $method" {} \; ; then
            echo "✓ $method - has inherent impl (in submodule)"
        else
            echo "✗ $method - MISSING inherent impl (RECURSION BUG!)"
            missing_impls+=("$method")
        fi
    fi
done

echo ""
echo "================================"
echo "Summary: ${#missing_impls[@]} methods lack inherent implementations"
if [ ${#missing_impls[@]} -gt 0 ]; then
    echo "Methods with potential recursion bugs:"
    for method in "${missing_impls[@]}"; do
        echo "  - $method"
    done
fi

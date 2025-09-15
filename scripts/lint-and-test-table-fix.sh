# Create clean ASCII table with proper formatting
echo "┌─────────────────────────────────────┬───────┬──────────┬─────────────────────────────────────────┐"
echo "│ Validation Category                 │ Count │ Status   │ Details / First Location                │"
echo "├─────────────────────────────────────┼───────┼──────────┼─────────────────────────────────────────┤"

# Anti-Pattern Detection
printf "│ %-35s │ %5d │ " "Database clones (total)" "$TOTAL_DATABASE_CLONES"
if [ "$PROBLEMATIC_DB_CLONES" -eq 0 ]; then
    printf "%-8s │ %-39s │\n" "✅ PASS" "${LEGITIMATE_ARC_CLONES} legitimate Arc clones"
else
    FIRST_DB_CLONE=$(get_first_location 'rg "\.as_ref\(\)\.clone\(\)|Arc::new\(database\.clone\(\)\)" src/ -g "!src/bin/*" -g "!src/database/tests.rs" -g "!src/database_plugins/*" -n')
    printf "%-8s │ %-39s │\n" "⚠️  WARN" "$FIRST_DB_CLONE"
fi

printf "│ %-35s │ %5d │ " "Resource creation patterns" "$RESOURCE_CREATION"
if [ "$RESOURCE_CREATION" -eq 0 ]; then
    printf "%-8s │ %-39s │\n" "✅ PASS" "Using dependency injection"
else
    FIRST_RESOURCE=$(get_first_location 'rg "AuthManager::new|OAuthManager::new|A2AClientManager::new|TenantOAuthManager::new" src/ -g "!src/mcp/multitenant.rs" -g "!src/mcp/resources.rs" -g "!src/bin/*" -g "!tests/*" -n')
    printf "%-8s │ %-39s │\n" "⚠️  WARN" "$FIRST_RESOURCE"
fi

printf "│ %-35s │ %5d │ " "Fake resource assemblies" "$FAKE_RESOURCES"
if [ "$FAKE_RESOURCES" -eq 0 ]; then
    printf "%-8s │ %-39s │\n" "✅ PASS" "No fake ServerResources"
else
    FIRST_FAKE=$(get_first_location 'rg "Arc::new\(ServerResources\s*\{" src/ -n')
    printf "%-8s │ %-39s │\n" "⚠️  WARN" "$FIRST_FAKE"
fi

printf "│ %-35s │ %5d │ " "Obsolete functions" "$OBSOLETE_FUNCTIONS"
if [ "$OBSOLETE_FUNCTIONS" -le 1 ]; then
    printf "%-8s │ %-39s │\n" "✅ PASS" "Within acceptable limits"
else
    FIRST_OBSOLETE=$(get_first_location 'rg "run_http_server\(" src/ -n')
    printf "%-8s │ %-39s │\n" "⚠️  WARN" "$FIRST_OBSOLETE"
fi

echo "├─────────────────────────────────────┼───────┼──────────┼─────────────────────────────────────────┤"

# Code Quality Analysis
printf "│ %-35s │ %5d │ " "Problematic unwraps" "$PROBLEMATIC_UNWRAPS"
if [ "$PROBLEMATIC_UNWRAPS" -eq 0 ]; then
    printf "%-8s │ %-39s │\n" "✅ PASS" "Proper error handling"
else
    FIRST_UNWRAP=$(get_first_location 'rg "\.unwrap\(\)" src/ | rg -v "// Safe|hardcoded.*valid|static.*data|00000000-0000-0000-0000-000000000000" -n')
    printf "%-8s │ %-39s │\n" "❌ FAIL" "$FIRST_UNWRAP"
fi

printf "│ %-35s │ %5d │ " "Problematic expects" "$PROBLEMATIC_EXPECTS"
if [ "$PROBLEMATIC_EXPECTS" -eq 0 ]; then
    printf "%-8s │ %-39s │\n" "✅ PASS" "Proper error handling"
else
    FIRST_EXPECT=$(get_first_location 'rg "\.expect\(" src/ | rg -v "// Safe|ServerResources.*required" -n')
    printf "%-8s │ %-39s │\n" "❌ FAIL" "$FIRST_EXPECT"
fi

printf "│ %-35s │ %5d │ " "Panic calls" "$PANICS"
if [ "$PANICS" -eq 0 ]; then
    printf "%-8s │ %-39s │\n" "✅ PASS" "No panic! found"
else
    FIRST_PANIC=$(get_first_location 'rg "panic!\(" src/ -n')
    printf "%-8s │ %-39s │\n" "❌ FAIL" "$FIRST_PANIC"
fi

printf "│ %-35s │ %5d │ " "TODOs/FIXMEs" "$TODOS"
if [ "$TODOS" -eq 0 ]; then
    printf "%-8s │ %-39s │\n" "✅ PASS" "No incomplete code"
else
    FIRST_TODO=$(get_first_location 'rg "TODO|FIXME|XXX" src/ -n')
    printf "%-8s │ %-39s │\n" "⚠️  WARN" "$FIRST_TODO"
fi

printf "│ %-35s │ %5d │ " "Placeholders/stubs" "$STUBS"
if [ "$STUBS" -eq 0 ]; then
    printf "%-8s │ %-39s │\n" "✅ PASS" "No stubs found"
else
    FIRST_STUB=$(get_first_location 'rg "stub|mock.*implementation" src/ -n')
    printf "%-8s │ %-39s │\n" "⚠️  WARN" "$FIRST_STUB"
fi

printf "│ %-35s │ %5d │ " "Problematic underscore names" "$PROBLEMATIC_UNDERSCORE_NAMES"
if [ "$PROBLEMATIC_UNDERSCORE_NAMES" -eq 0 ]; then
    printf "%-8s │ %-39s │\n" "✅ PASS" "Good naming conventions"
else
    FIRST_UNDERSCORE=$(get_first_location 'rg "fn _|let _[a-zA-Z]|struct _|enum _" src/ | rg -v "let _[[:space:]]*=" | rg -v "let _result|let _response|let _output" -n')
    printf "%-8s │ %-39s │\n" "⚠️  WARN" "$FIRST_UNDERSCORE"
fi

printf "│ %-35s │ %5d │ " "Example emails" "$EXAMPLE_EMAILS"
if [ "$EXAMPLE_EMAILS" -eq 0 ]; then
    printf "%-8s │ %-39s │\n" "✅ PASS" "No test emails in production"
else
    FIRST_EMAIL=$(get_first_location 'rg "example\.com|test@" src/ -g "!src/bin/*" -n')
    printf "%-8s │ %-39s │\n" "⚠️  INFO" "$FIRST_EMAIL"
fi

printf "│ %-35s │ %5d │ " "Temporary solutions" "$TEMP_SOLUTIONS"
if [ "$TEMP_SOLUTIONS" -eq 0 ]; then
    printf "%-8s │ %-39s │\n" "✅ PASS" "No temporary code"
else
    FIRST_TEMP=$(get_first_location 'rg "\bhack\b|\bworkaround\b|\bquick.*fix\b|future.*implementation|temporary.*solution|temp.*fix" src/ -n')
    printf "%-8s │ %-39s │\n" "⚠️  WARN" "$FIRST_TEMP"
fi

echo "├─────────────────────────────────────┼───────┼──────────┼─────────────────────────────────────────┤"

# Memory Management Analysis
printf "│ %-35s │ %5d │ " "Clone usage" "$TOTAL_CLONES"
if [ "$TOTAL_CLONES" -lt 500 ]; then
    printf "%-8s │ %-39s │\n" "✅ PASS" "Mostly legitimate Arc/String clones"
else
    FIRST_PROBLEMATIC_CLONE=$(get_first_location 'rg "\.clone\(\)" src/ | rg -v "Arc::|resources\.|database\.|auth_manager\.|\.to_string\(\)|format!|String::from|token|url|name|path|message|error|Error" -n')
    printf "%-8s │ %-39s │\n" "⚠️  WARN" "$FIRST_PROBLEMATIC_CLONE"
fi

printf "│ %-35s │ %5d │ " "Arc usage" "$TOTAL_ARCS"
if [ "$TOTAL_ARCS" -lt 50 ]; then
    printf "%-8s │ %-39s │\n" "✅ PASS" "Appropriate for service architecture"
else
    FIRST_PROBLEMATIC_ARC=$(get_first_location 'rg "Arc::" src/ | rg -v "ServerResources|Manager|Executor|Lock|Mutex|RwLock" -n')
    printf "%-8s │ %-39s │\n" "⚠️  WARN" "$FIRST_PROBLEMATIC_ARC"
fi

printf "│ %-35s │ %5d │ " "Magic numbers" "$MAGIC_NUMBERS"
if [ "$MAGIC_NUMBERS" -lt 10 ]; then
    printf "%-8s │ %-39s │\n" "✅ PASS" "Good configuration practices"
else
    FIRST_MAGIC=$(get_first_location 'rg "\b[0-9]{4,}\b" src/ -g "!src/constants.rs" -g "!src/config/*" | grep -v -E "(Licensed|http://|https://|Duration|timestamp|//.*[0-9]|seconds|minutes|hours|Version|\.[0-9]|[0-9]\.|test|mock|example|error.*code|status.*code|port|timeout|limit|capacity|-32[0-9]{3}|1000\.0|60\.0|24\.0|7\.0|365\.0|METERS_PER|PER_METER|conversion|unit|\.60934|12345|0000-0000|202[0-9]-[0-9]{2}-[0-9]{2}|Some\([0-9]+\)|Trial.*1000|Standard.*10000)" -n')
    printf "%-8s │ %-39s │\n" "⚠️  WARN" "$FIRST_MAGIC"
fi

echo "└─────────────────────────────────────┴───────┴──────────┴─────────────────────────────────────────┘"
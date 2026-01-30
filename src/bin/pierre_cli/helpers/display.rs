// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence
// ABOUTME: Output formatting helpers for pierre-cli
// ABOUTME: Provides consistent display functions for tokens and user information

use pierre_mcp_server::admin::models::GeneratedAdminToken;

/// Display a generated token with important security warnings
pub fn display_generated_token(token: &GeneratedAdminToken) {
    println!("\nComplete Admin Token Generated Successfully!");
    println!("{}", "=".repeat(80));
    println!("Key TOKEN DETAILS:");
    println!("   Service: {}", token.service_name);
    println!("   Token ID: {}", token.token_id);
    println!(
        "   Super Admin: {}",
        if token.is_super_admin {
            "Success Yes"
        } else {
            "Error No"
        }
    );

    if let Some(expires) = token.expires_at {
        println!("   Expires: {}", expires.format("%Y-%m-%d %H:%M UTC"));
    } else {
        println!("   Expires: Never");
    }

    println!("\nKey YOUR JWT TOKEN (SAVE THIS NOW):");
    println!("{}", "=".repeat(80));
    println!("{}", token.jwt_token);
    println!("{}", "=".repeat(80));

    println!("\nWARNING CRITICAL SECURITY NOTES:");
    println!("• This token is shown ONLY ONCE - save it now!");
    println!("• Store it securely in your admin service environment:");
    println!("  export PIERRE_MCP_ADMIN_TOKEN=\"{}\"", token.jwt_token);
    println!("• Never share this token or commit it to version control");
    println!("• Use this token in Authorization header: Bearer <token>");
    println!(
        "• This token allows {} API key operations",
        if token.is_super_admin {
            "ALL"
        } else {
            "limited"
        }
    );

    println!("\nDocs NEXT STEPS:");
    println!("1. Save the token to your admin service environment");
    println!("2. Configure your admin service to use this token");
    println!("3. Test the connection with your admin service");

    if !token.is_super_admin {
        println!("4. For super admin access, use --super-admin flag");
    }

    println!("\nUSAGE EXAMPLE:");
    println!("curl -H \"Authorization: Bearer {}\" \\", token.jwt_token);
    println!("     -X POST http://localhost:8080/admin/provision-api-key \\");
    println!("     -d '{{\"user_email\":\"user@example.com\",\"tier\":\"starter\"}}'");

    println!("\nSuccess Token is ready to use!");
}

/// Display admin user creation success message
pub fn display_admin_user_success(email: &str, name: &str, password: &str, super_admin: bool) {
    let role_str = if super_admin { "Super Admin" } else { "Admin" };
    println!("\nSuccess {role_str} User Created Successfully!");
    println!("{}", "=".repeat(50));
    println!("User USER DETAILS:");
    println!("   Email: {email}");
    println!("   Name: {name}");
    println!("   Role: {role_str}");
    println!("   Tier: Enterprise (Full access)");
    println!("   Status: Active");

    if super_admin {
        println!("   Capabilities: Can impersonate other users");
    }

    println!("\nKey LOGIN CREDENTIALS:");
    println!("{}", "=".repeat(50));
    println!("   Email: {email}");
    println!("   Password: {password}");

    println!("\nWARNING IMPORTANT SECURITY NOTES:");
    println!("• Change the default password in production!");
    println!("• This user has full access to the admin interface");
    if super_admin {
        println!("• Super admin can impersonate any non-super-admin user");
    }
    println!("• Use strong passwords and enable 2FA if available");
    println!("• Consider creating additional admin users with limited permissions");

    println!("\nDocs NEXT STEPS:");
    println!("1. Start the Pierre MCP Server:");
    println!("   cargo run --bin pierre-mcp-server");
    println!("2. Open the frontend interface (usually http://localhost:8080)");
    println!("3. Login with the credentials above");
    println!("4. Generate your first admin token for API key provisioning");
}

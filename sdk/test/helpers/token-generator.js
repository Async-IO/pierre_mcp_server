// ABOUTME: JWT token generator for automated testing without browser OAuth
// ABOUTME: Creates valid test tokens with configurable claims and expiration
//
// Licensed under either of Apache License, Version 2.0 or MIT License at your option.
// Copyright ©2025 Async-IO.org

const { spawn } = require('child_process');
const crypto = require('crypto');
const fs = require('fs');
const path = require('path');
const os = require('os');

/**
 * Generate a valid JWT token for testing
 * This mimics what the Pierre server would generate after OAuth
 */
function generateTestToken(userId, email, expiresIn = 3600) {
    // JWT structure: header.payload.signature
    const header = {
        typ: 'JWT',
        alg: 'HS256'
    };

    const now = Math.floor(Date.now() / 1000);
    const payload = {
        sub: userId,
        email: email,
        iat: now,
        exp: now + expiresIn,
        providers: [] // No providers connected initially
    };

    // Base64url encode
    const base64Header = Buffer.from(JSON.stringify(header)).toString('base64url');
    const base64Payload = Buffer.from(JSON.stringify(payload)).toString('base64url');

    // For testing, we use a known test secret
    // In production, this comes from PIERRE_JWT_SECRET environment variable
    const testSecret = process.env.PIERRE_JWT_SECRET || 'test_jwt_secret_for_automated_tests_only';

    // Create signature
    const signatureInput = `${base64Header}.${base64Payload}`;
    const signature = crypto
        .createHmac('sha256', testSecret)
        .update(signatureInput)
        .digest('base64url');

    const token = `${base64Header}.${base64Payload}.${signature}`;

    return {
        access_token: token,
        token_type: 'Bearer',
        expires_in: expiresIn,
        scope: 'read:fitness write:fitness',
        saved_at: now
    };
}

/**
 * Create a test user via admin-setup and return user ID
 */
async function createTestUser(options = {}) {
    const email = options.email || `test-${Date.now()}@example.com`;
    const password = options.password || 'TestPassword123!';
    const name = options.name || 'Test User';

    const projectRoot = path.join(__dirname, '../../..');
    const binaryPath = path.join(projectRoot, 'target/debug/admin-setup');

    // Check if binary exists, build if not
    if (!fs.existsSync(binaryPath)) {
        console.log('Building admin-setup binary...');
        await runCommand('cargo', ['build', '--bin', 'admin-setup'], { cwd: projectRoot });
    }

    const env = {
        ...process.env,
        DATABASE_URL: options.databaseUrl || process.env.DATABASE_URL || 'sqlite::memory:',
        PIERRE_MASTER_ENCRYPTION_KEY: options.encryptionKey || process.env.PIERRE_MASTER_ENCRYPTION_KEY || 'rEFe91l6lqLahoyl9OSzum9dKa40VvV5RYj8bHGNTeo='
    };

    // Run create-admin-user command
    const output = await runCommand(
        binaryPath,
        [
            'create-admin-user',
            '--email', email,
            '--password', password,
            '--name', name,
            '--force'
        ],
        { env, cwd: projectRoot }
    );

    // Extract user ID from output (admin-setup should print it)
    // If not, generate a UUID for testing
    const uuidMatch = output.match(/[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}/i);
    const userId = uuidMatch ? uuidMatch[0] : crypto.randomUUID();

    return {
        userId,
        email,
        password,
        name
    };
}

/**
 * Generate and write a complete token file for testing
 */
async function setupTestToken(options = {}) {
    const tokenFile = options.tokenFile || path.join(os.homedir(), '.pierre-claude-tokens.json');

    // Create or get test user
    let userId, email;
    if (options.userId && options.email) {
        userId = options.userId;
        email = options.email;
    } else {
        const user = await createTestUser(options);
        userId = user.userId;
        email = user.email;
    }

    // Generate Pierre token
    const pierreToken = generateTestToken(userId, email, options.expiresIn || 3600);

    // Create token file structure
    const tokens = {
        pierre: pierreToken,
        providers: {}
    };

    // Add provider tokens if specified
    if (options.providers) {
        tokens.providers = options.providers;
    }

    // Write to file
    fs.writeFileSync(tokenFile, JSON.stringify(tokens, null, 2));

    console.log(`✅ Test token created for user: ${email}`);
    console.log(`   User ID: ${userId}`);
    console.log(`   Token file: ${tokenFile}`);
    console.log(`   Expires in: ${options.expiresIn || 3600}s`);

    return {
        userId,
        email,
        tokenFile,
        tokens
    };
}

/**
 * Run a command and return output
 */
function runCommand(command, args, options = {}) {
    return new Promise((resolve, reject) => {
        const proc = spawn(command, args, {
            ...options,
            stdio: 'pipe'
        });

        let stdout = '';
        let stderr = '';

        proc.stdout.on('data', (data) => {
            stdout += data.toString();
        });

        proc.stderr.on('data', (data) => {
            stderr += data.toString();
        });

        proc.on('close', (code) => {
            if (code !== 0) {
                reject(new Error(`Command failed with code ${code}:\n${stderr}`));
            } else {
                resolve(stdout + stderr);
            }
        });

        proc.on('error', reject);
    });
}

module.exports = {
    generateTestToken,
    createTestUser,
    setupTestToken
};

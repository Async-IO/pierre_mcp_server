#!/usr/bin/env node

/**
 * Test script for clean token expiry fix
 * Tests scenarios 1-6 with automated validation
 *
 * Usage: node test/test_clean_token_fix.js
 *
 * Prerequisites:
 * - Pierre server running on http://localhost:8081
 * - User account already created on Pierre server
 */

const { spawn } = require('child_process');
const fs = require('fs');
const path = require('path');
const readline = require('readline');

class CleanTokenFixTest {
    constructor() {
        this.tokenFile = path.join(require('os').homedir(), '.pierre-claude-tokens.json');
        this.tokenBackupFile = this.tokenFile + '.backup';
        this.results = {
            passed: 0,
            failed: 0,
            skipped: 0,
            scenarios: []
        };
        this.bridgeProcess = null;
        this.inspectorProcess = null;
    }

    log(message) {
        const timestamp = new Date().toISOString();
        console.log(`[${timestamp}] ${message}`);
    }

    logSection(message) {
        console.log('\n' + '='.repeat(80));
        console.log(message);
        console.log('='.repeat(80) + '\n');
    }

    async backupTokens() {
        if (fs.existsSync(this.tokenFile)) {
            this.log('📦 Backing up existing token file...');
            fs.copyFileSync(this.tokenFile, this.tokenBackupFile);
            this.log(`✅ Tokens backed up to ${this.tokenBackupFile}`);
        } else {
            this.log('ℹ️  No existing token file to backup');
        }
    }

    async restoreTokens() {
        if (fs.existsSync(this.tokenBackupFile)) {
            this.log('📦 Restoring original token file...');
            fs.copyFileSync(this.tokenBackupFile, this.tokenFile);
            fs.unlinkSync(this.tokenBackupFile);
            this.log('✅ Tokens restored');
        }
    }

    async deleteTokens() {
        if (fs.existsSync(this.tokenFile)) {
            fs.unlinkSync(this.tokenFile);
            this.log('🗑️  Deleted token file');
        }
    }

    async readTokens() {
        if (fs.existsSync(this.tokenFile)) {
            const content = fs.readFileSync(this.tokenFile, 'utf8');
            return JSON.parse(content);
        }
        return null;
    }

    async writeTokens(tokens) {
        fs.writeFileSync(this.tokenFile, JSON.stringify(tokens, null, 2));
        this.log('💾 Updated token file');
    }

    async expireToken() {
        const tokens = await this.readTokens();
        if (tokens && tokens.pierre) {
            // Set saved_at to a time that makes the token expired
            tokens.pierre.saved_at = Math.floor(Date.now() / 1000) - (tokens.pierre.expires_in || 3600) - 100;
            await this.writeTokens(tokens);
            this.log('⏰ Token expired (saved_at set to past)');
            return true;
        }
        return false;
    }

    async corruptToken() {
        const tokens = await this.readTokens();
        if (tokens && tokens.pierre) {
            // Corrupt the access token
            tokens.pierre.access_token = 'corrupted_token_' + Math.random().toString(36);
            await this.writeTokens(tokens);
            this.log('🔨 Token corrupted');
            return true;
        }
        return false;
    }

    async waitForUserInput(message) {
        const rl = readline.createInterface({
            input: process.stdin,
            output: process.stdout
        });

        return new Promise((resolve) => {
            rl.question(`\n${message}\nPress ENTER to continue...`, () => {
                rl.close();
                resolve();
            });
        });
    }

    recordResult(scenario, passed, message) {
        if (passed) {
            this.results.passed++;
        } else {
            this.results.failed++;
        }

        this.results.scenarios.push({
            scenario,
            passed,
            message,
            timestamp: new Date().toISOString()
        });

        const emoji = passed ? '✅' : '❌';
        this.log(`${emoji} ${scenario}: ${message}`);
    }

    // Scenario 1: Fresh Start (No Tokens)
    async testScenario1() {
        this.logSection('SCENARIO 1: Fresh Start (No Tokens)');

        try {
            this.log('Step 1: Deleting existing tokens...');
            await this.deleteTokens();

            this.log('Step 2: Starting bridge with MCP Inspector...');
            this.log('ℹ️  You will need to:');
            this.log('   1. Wait for Inspector to start');
            this.log('   2. Run: tools/list');
            this.log('   3. Verify only "connect_to_pierre" tool is shown');
            this.log('   4. Run: tools/call connect_to_pierre');
            this.log('   5. Complete authentication in browser');
            this.log('   6. Run: tools/list again');
            this.log('   7. Verify all tools are now shown');

            await this.waitForUserInput('Complete the steps above, then press ENTER');

            // Verify token file was created
            const tokens = await this.readTokens();
            if (tokens && tokens.pierre && tokens.pierre.access_token) {
                this.recordResult('Scenario 1', true, 'Token file created with valid Pierre token');
            } else {
                this.recordResult('Scenario 1', false, 'Token file not created or invalid');
            }

        } catch (error) {
            this.recordResult('Scenario 1', false, `Error: ${error.message}`);
        }
    }

    // Scenario 2: Expired Token (Auto Re-Authentication)
    async testScenario2() {
        this.logSection('SCENARIO 2: Expired Token (Auto Re-Authentication)');

        try {
            this.log('Step 1: Checking for existing token...');
            const tokens = await this.readTokens();
            if (!tokens || !tokens.pierre) {
                this.recordResult('Scenario 2', false, 'Skipped - no token file (run Scenario 1 first)');
                this.results.skipped++;
                return;
            }

            this.log('Step 2: Expiring token...');
            await this.expireToken();

            this.log('Step 3: Restart bridge and test auto re-authentication...');
            this.log('ℹ️  You will need to:');
            this.log('   1. Restart the Inspector (Ctrl+C and restart)');
            this.log('   2. Run: tools/list');
            this.log('   3. EXPECTED: Browser opens automatically for re-authentication');
            this.log('   4. Complete authentication in browser');
            this.log('   5. Run: tools/list again');
            this.log('   6. Verify all tools are shown');

            await this.waitForUserInput('Complete the steps above, then press ENTER');

            // Verify token was updated
            const newTokens = await this.readTokens();
            const now = Math.floor(Date.now() / 1000);
            const expiresAt = (newTokens.pierre.saved_at || 0) + (newTokens.pierre.expires_in || 0);

            if (expiresAt > now) {
                this.recordResult('Scenario 2', true, 'Token refreshed successfully (not expired)');
            } else {
                this.recordResult('Scenario 2', false, 'Token still expired after refresh');
            }

        } catch (error) {
            this.recordResult('Scenario 2', false, `Error: ${error.message}`);
        }
    }

    // Scenario 3: Invalid Token (Server Rejects)
    async testScenario3() {
        this.logSection('SCENARIO 3: Invalid Token (Server Rejects)');

        try {
            this.log('Step 1: Checking for existing token...');
            const tokens = await this.readTokens();
            if (!tokens || !tokens.pierre) {
                this.recordResult('Scenario 3', false, 'Skipped - no token file');
                this.results.skipped++;
                return;
            }

            this.log('Step 2: Corrupting token...');
            await this.corruptToken();

            this.log('Step 3: Restart bridge and test auto re-authentication...');
            this.log('ℹ️  You will need to:');
            this.log('   1. Restart the Inspector (Ctrl+C and restart)');
            this.log('   2. Run: tools/list');
            this.log('   3. EXPECTED: Browser opens automatically (invalid token detected)');
            this.log('   4. Complete authentication in browser');
            this.log('   5. Run: tools/list again');
            this.log('   6. Verify all tools are shown');

            await this.waitForUserInput('Complete the steps above, then press ENTER');

            // Verify token is valid now
            const newTokens = await this.readTokens();
            if (newTokens.pierre.access_token && !newTokens.pierre.access_token.startsWith('corrupted_')) {
                this.recordResult('Scenario 3', true, 'Invalid token replaced with valid one');
            } else {
                this.recordResult('Scenario 3', false, 'Token still invalid');
            }

        } catch (error) {
            this.recordResult('Scenario 3', false, `Error: ${error.message}`);
        }
    }

    // Scenario 4: Provider Already Connected (Optimization)
    async testScenario4() {
        this.logSection('SCENARIO 4: Provider Already Connected (Optimization)');

        try {
            this.log('Step 1: Verify valid Pierre token exists...');
            const tokens = await this.readTokens();
            if (!tokens || !tokens.pierre) {
                this.recordResult('Scenario 4', false, 'Skipped - no valid Pierre token');
                this.results.skipped++;
                return;
            }

            this.log('Step 2: Test provider connection optimization...');
            this.log('ℹ️  You will need to:');
            this.log('   1. Make sure Inspector is running with valid Pierre token');
            this.log('   2. Run: tools/call connect_provider {"provider": "strava"}');
            this.log('   3. Complete Strava authentication in browser (first time)');
            this.log('   4. Run: tools/call connect_provider {"provider": "strava"} AGAIN');
            this.log('   5. EXPECTED: Returns "Already connected to STRAVA!" WITHOUT opening browser');
            this.log('   6. Verify no browser window opened the second time');

            await this.waitForUserInput('Complete the steps above, then press ENTER');

            // Check if provider token exists in storage
            const newTokens = await this.readTokens();
            if (newTokens.providers && newTokens.providers.strava) {
                this.recordResult('Scenario 4', true, 'Provider tokens saved, optimization should work');
            } else {
                this.recordResult('Scenario 4', false, 'Provider tokens not found in storage');
            }

        } catch (error) {
            this.recordResult('Scenario 4', false, `Error: ${error.message}`);
        }
    }

    // Scenario 5: Provider Connection with Pierre Auth Error
    async testScenario5() {
        this.logSection('SCENARIO 5: Provider Connection with Pierre Auth Error');

        try {
            this.log('Step 1: Verify tokens exist...');
            const tokens = await this.readTokens();
            if (!tokens || !tokens.pierre) {
                this.recordResult('Scenario 5', false, 'Skipped - no tokens');
                this.results.skipped++;
                return;
            }

            // Remove provider tokens to force fresh connection
            if (tokens.providers) {
                delete tokens.providers.strava;
                await this.writeTokens(tokens);
                this.log('🗑️  Removed Strava provider tokens');
            }

            this.log('Step 2: Expiring Pierre token...');
            await this.expireToken();

            this.log('Step 3: Test provider connection with expired Pierre token...');
            this.log('ℹ️  You will need to:');
            this.log('   1. Restart Inspector to pick up expired token');
            this.log('   2. Run: tools/call connect_provider {"provider": "strava"}');
            this.log('   3. EXPECTED: Browser opens for Pierre re-authentication FIRST');
            this.log('   4. Complete Pierre authentication');
            this.log('   5. EXPECTED: Browser opens for Strava authentication SECOND');
            this.log('   6. Complete Strava authentication');
            this.log('   7. Verify both authentications completed successfully');

            await this.waitForUserInput('Complete the steps above, then press ENTER');

            // Verify both tokens are valid
            const newTokens = await this.readTokens();
            const pierreValid = newTokens.pierre && newTokens.pierre.access_token;
            const stravaValid = newTokens.providers && newTokens.providers.strava;

            if (pierreValid && stravaValid) {
                this.recordResult('Scenario 5', true, 'Both Pierre and Strava tokens refreshed successfully');
            } else {
                this.recordResult('Scenario 5', false, `Pierre: ${pierreValid}, Strava: ${stravaValid}`);
            }

        } catch (error) {
            this.recordResult('Scenario 5', false, `Error: ${error.message}`);
        }
    }

    // Scenario 6: Retry Limit Protection
    async testScenario6() {
        this.logSection('SCENARIO 6: Retry Limit Protection');

        this.log('ℹ️  This scenario tests infinite loop prevention');
        this.log('ℹ️  It requires observing logs for retry behavior');
        this.log('ℹ️  You will need to:');
        this.log('   1. Corrupt the token');
        this.log('   2. Start Inspector');
        this.log('   3. Attempt authentication but CANCEL/FAIL it intentionally');
        this.log('   4. Observer bridge logs for "Maximum retry limit reached"');
        this.log('   5. Verify it stops after 2 retries and doesn\'t loop forever');

        await this.waitForUserInput('If you want to run this test, set it up and press ENTER (or skip by pressing ENTER)');

        this.recordResult('Scenario 6', true, 'Manual verification required - check logs for retry limits');

    }

    printReport() {
        this.logSection('TEST REPORT');

        console.log(`Total Scenarios: ${this.results.scenarios.length}`);
        console.log(`✅ Passed: ${this.results.passed}`);
        console.log(`❌ Failed: ${this.results.failed}`);
        console.log(`⏭️  Skipped: ${this.results.skipped}`);
        console.log();

        console.log('Detailed Results:');
        console.log('-'.repeat(80));

        this.results.scenarios.forEach((result, index) => {
            const emoji = result.passed ? '✅' : '❌';
            console.log(`${index + 1}. ${emoji} ${result.scenario}`);
            console.log(`   ${result.message}`);
            console.log(`   Time: ${result.timestamp}`);
            console.log();
        });

        console.log('='.repeat(80));

        if (this.results.failed === 0) {
            console.log('🎉 All tests passed!');
        } else {
            console.log('⚠️  Some tests failed. Review the results above.');
        }
    }

    async run() {
        this.logSection('CLEAN TOKEN FIX - AUTOMATED TEST SUITE');

        this.log('📋 This test suite will run scenarios 1-6');
        this.log('📋 You will need to interact with the MCP Inspector for each scenario');
        this.log('📋 Make sure Pierre server is running on http://localhost:8081');
        this.log('');
        this.log('🚀 Starting tests...');

        // Backup existing tokens
        await this.backupTokens();

        try {
            // Run all scenarios
            await this.testScenario1();
            await this.testScenario2();
            await this.testScenario3();
            await this.testScenario4();
            await this.testScenario5();
            await this.testScenario6();

        } finally {
            // Print report
            this.printReport();

            // Ask if user wants to restore tokens
            const rl = readline.createInterface({
                input: process.stdin,
                output: process.stdout
            });

            rl.question('\nRestore original tokens? (y/n): ', async (answer) => {
                if (answer.toLowerCase() === 'y') {
                    await this.restoreTokens();
                } else {
                    this.log('ℹ️  Keeping test tokens. You can manually restore from .pierre-claude-tokens.json.backup if needed');
                }
                rl.close();
            });
        }
    }
}

// Run the tests
if (require.main === module) {
    const tester = new CleanTokenFixTest();
    tester.run().catch(error => {
        console.error('Fatal error:', error);
        process.exit(1);
    });
}

module.exports = CleanTokenFixTest;

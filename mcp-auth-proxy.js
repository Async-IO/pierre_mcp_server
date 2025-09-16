#!/usr/bin/env node

// ABOUTME: OAuth authentication proxy for Claude Desktop MCP integration
// ABOUTME: Handles OAuth flows and token management for Pierre MCP Server

const express = require('express');
const { spawn } = require('child_process');
const cors = require('cors');

const app = express();
const PORT = process.env.MCP_AUTH_PROXY_PORT || 3000;
const PIERRE_SERVER_URL = process.env.PIERRE_SERVER_URL || 'http://localhost:8080';

app.use(cors());
app.use(express.json());

// Health check endpoint
app.get('/health', (req, res) => {
    res.json({ status: 'ok', service: 'pierre-mcp-auth-proxy' });
});

// OAuth callback handler
app.get('/auth/:provider/callback', async (req, res) => {
    const { provider } = req.params;
    const { code, state } = req.query;

    console.log(`OAuth callback for ${provider}: code=${code?.substring(0, 10)}..., state=${state}`);

    try {
        // Forward to Pierre server
        const response = await fetch(`${PIERRE_SERVER_URL}/api/oauth/callback/${provider}`, {
            method: 'GET',
            headers: {
                'Content-Type': 'application/json',
            },
            // Forward the query parameters
            body: JSON.stringify({ code, state })
        });

        if (response.ok) {
            res.send('OAuth flow completed successfully. You can close this window.');
        } else {
            const error = await response.text();
            res.status(response.status).send(`OAuth error: ${error}`);
        }
    } catch (error) {
        console.error('OAuth callback error:', error);
        res.status(500).send(`OAuth callback failed: ${error.message}`);
    }
});

// MCP stdio proxy
app.post('/mcp', async (req, res) => {
    try {
        // Forward MCP requests to Pierre server
        const response = await fetch(`${PIERRE_SERVER_URL}/mcp`, {
            method: 'POST',
            headers: {
                'Content-Type': 'application/json',
                'Authorization': req.headers.authorization || '',
            },
            body: JSON.stringify(req.body)
        });

        const result = await response.json();
        res.json(result);
    } catch (error) {
        console.error('MCP proxy error:', error);
        res.status(500).json({
            jsonrpc: '2.0',
            error: {
                code: -32603,
                message: `MCP proxy error: ${error.message}`
            },
            id: req.body?.id || null
        });
    }
});

app.listen(PORT, () => {
    console.log(`Pierre MCP Auth Proxy listening on port ${PORT}`);
    console.log(`Proxying to Pierre server: ${PIERRE_SERVER_URL}`);
    console.log(`OAuth callback URL: http://localhost:${PORT}/auth/{provider}/callback`);
});
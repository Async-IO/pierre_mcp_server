#!/usr/bin/env node
// Pierre MCP Client for Claude Desktop
// Download from: https://github.com/your-org/pierre-mcp-client
// 
// Setup:
// 1. Replace YOUR_API_KEY with your actual API key
// 2. Save this file as pierre-mcp-client.js  
// 3. Run: chmod +x pierre-mcp-client.js
// 4. Add to Claude Desktop config

const http = require('http');
const https = require('https');
const { URL } = require('url');
const readline = require('readline');

// Configuration - Replace with your values
const API_KEY = 'YOUR_API_KEY';  // Replace with your actual API key
const SERVER_URL = 'https://pierre-mcp.your-domain.com/mcp';  // Production server URL

const rl = readline.createInterface({
  input: process.stdin,
  output: process.stdout,
  terminal: false
});

rl.on('line', (line) => {
  const url = new URL(SERVER_URL);
  const client = url.protocol === 'https:' ? https : http;
  
  const options = {
    hostname: url.hostname,
    port: url.port || (url.protocol === 'https:' ? 443 : 80),
    path: url.pathname,
    method: 'POST',
    headers: {
      'Authorization': API_KEY,
      'Content-Type': 'application/json',
      'Content-Length': Buffer.byteLength(line)
    }
  };

  const req = client.request(options, (res) => {
    let data = '';
    res.on('data', (chunk) => data += chunk);
    res.on('end', () => process.stdout.write(data + '\n'));
  });

  req.on('error', (e) => {
    const error = {
      jsonrpc: '2.0',
      id: null,
      error: { code: -32603, message: `Connection failed: ${e.message}` }
    };
    process.stdout.write(JSON.stringify(error) + '\n');
  });

  req.write(line);
  req.end();
});
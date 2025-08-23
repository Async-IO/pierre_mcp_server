#!/usr/bin/env node
// ABOUTME: HTTP MCP client for Claude Desktop integration with Pierre MCP Server
// ABOUTME: Provides production-ready HTTP transport without local bridges

const http = require('http');
const https = require('https');
const { URL } = require('url');
const readline = require('readline');

const API_KEY = process.env.PIERRE_API_KEY || 'YOUR_API_KEY_HERE';
const SERVER_URL = process.env.PIERRE_MCP_SERVER_URL || 'http://localhost:8080/mcp';

// Parse server URL to get protocol, host, port, and path
const url = new URL(SERVER_URL);
const isHttps = url.protocol === 'https:';
const client = isHttps ? https : http;
const SERVER_HOST = url.hostname;
const SERVER_PORT = parseInt(url.port) || (isHttps ? 443 : 80);
const SERVER_PATH = url.pathname;

const rl = readline.createInterface({
  input: process.stdin,
  output: process.stdout,
  terminal: false
});

rl.on('line', (line) => {
  const options = {
    hostname: SERVER_HOST,
    port: SERVER_PORT,
    path: SERVER_PATH,
    method: 'POST',
    headers: {
      'Authorization': `Bearer ${API_KEY}`,
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
      error: { code: -32603, message: e.message }
    };
    process.stdout.write(JSON.stringify(error) + '\n');
  });

  req.write(line);
  req.end();
});
#!/usr/bin/env node
// ABOUTME: HTTP MCP client for Claude Desktop integration with Pierre MCP Server
// ABOUTME: Provides production-ready HTTP transport without local bridges

const http = require('http');
const readline = require('readline');

const API_KEY = 'pk_live_akODQjOwjSStpvyRd0GQvSEvEyfklRNy';
const SERVER_HOST = 'localhost';
const SERVER_PORT = 8080;
const SERVER_PATH = '/mcp';

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
      'Authorization': API_KEY,
      'Content-Type': 'application/json',
      'Content-Length': Buffer.byteLength(line)
    }
  };

  const req = http.request(options, (res) => {
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
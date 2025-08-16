#!/usr/bin/env node

// Pierre MCP Bridge for Claude Desktop
// Translates between Claude Desktop MCP protocol and Pierre HTTP server

const http = require('http');
const readline = require('readline');

const API_KEY = 'pk_live_akODQjOwjSStpvyRd0GQvSEvEyfklRNy';
const SERVER_HOST = 'localhost';
const SERVER_PORT = 8080;

// Debug logging to stderr (visible in Claude Desktop logs)
function debug(msg) {
  console.error(`[Pierre Bridge] ${msg}`);
}

debug(`Starting bridge with API key: ${API_KEY.substring(0, 20)}...`);

const rl = readline.createInterface({
  input: process.stdin,
  output: process.stdout,
  terminal: false
});

rl.on('line', async (line) => {
  debug(`Received: ${line.substring(0, 100)}...`);
  
  try {
    const request = JSON.parse(line);
    debug(`Parsed request method: ${request.method}, id: ${request.id}`);
    
    // Forward request to Pierre server
    const postData = JSON.stringify(request);
    
    const options = {
      hostname: SERVER_HOST,
      port: SERVER_PORT,
      path: '/mcp',
      method: 'POST',
      headers: {
        'Authorization': API_KEY,
        'Content-Type': 'application/json',
        'Content-Length': Buffer.byteLength(postData)
      }
    };
    
    const req = http.request(options, (res) => {
      let data = '';
      
      res.on('data', (chunk) => {
        data += chunk;
      });
      
      res.on('end', () => {
        debug(`Server response status: ${res.statusCode}`);
        debug(`Server response: ${data.substring(0, 200)}...`);
        
        // Forward the response directly
        process.stdout.write(data + '\n');
      });
    });
    
    req.on('error', (e) => {
      debug(`Connection error: ${e.message}`);
      const errorResponse = {
        jsonrpc: '2.0',
        id: request.id || null,
        error: { 
          code: -32603, 
          message: `Connection to Pierre server failed: ${e.message}` 
        }
      };
      process.stdout.write(JSON.stringify(errorResponse) + '\n');
    });
    
    req.write(postData);
    req.end();
  } catch (e) {
    debug(`Parse error: ${e.message}`);
    // Parse error - still need proper JSON-RPC response
    const errorResponse = {
      jsonrpc: '2.0',
      id: null,
      error: { 
        code: -32700, 
        message: `Parse error: ${e.message}` 
      }
    };
    process.stdout.write(JSON.stringify(errorResponse) + '\n');
  }
});

// Handle process lifecycle
process.on('SIGINT', () => {
  debug('Received SIGINT, shutting down');
  process.exit(0);
});

process.on('SIGTERM', () => {
  debug('Received SIGTERM, shutting down');
  process.exit(0);
});

process.stdin.on('end', () => {
  debug('stdin closed, shutting down');
  process.exit(0);
});

debug('Bridge ready, waiting for input...');
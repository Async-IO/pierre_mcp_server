#!/usr/bin/env node
const http = require('http');
const readline = require('readline');

const API_KEY = 'pk_live_L2Q5HCWtDGl8tLvZXILnNmyPSnqynHg7';

const rl = readline.createInterface({
  input: process.stdin,
  output: process.stdout,
  terminal: false
});

console.error('[Pierre Bridge] Starting with API Key: ' + API_KEY.substring(0, 15) + '...');

rl.on('line', async (line) => {
  console.error('[Pierre Bridge] Received: ' + line.substring(0, 100) + '...');
  
  try {
    const request = JSON.parse(line);
    
    // Handle initialize request specially
    if (request.method === 'initialize') {
      console.error('[Pierre Bridge] Handling initialize request');
      const initResponse = {
        jsonrpc: '2.0',
        id: request.id,
        result: {
          protocolVersion: '2025-06-18',
          serverInfo: {
            name: 'pierre-fitness',
            version: '0.1.0'
          },
          capabilities: {
            tools: {
              list: true
            }
          }
        }
      };
      console.error('[Pierre Bridge] Sending init response: ' + JSON.stringify(initResponse));
      process.stdout.write(JSON.stringify(initResponse) + '\n');
      return;
    }
    
    // Forward all other requests to Pierre server
    const postData = JSON.stringify(request);
    
    const options = {
      hostname: 'localhost',
      port: 8080,
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
        try {
          const result = JSON.parse(data);
          console.error('[Pierre Bridge] Server response: ' + JSON.stringify(result).substring(0, 200) + '...');
          
          // Make sure response has proper structure
          const response = {
            jsonrpc: '2.0',
            id: request.id
          };
          
          if (result.result !== undefined) {
            response.result = result.result;
          } else if (result.error !== undefined) {
            response.error = result.error;
          } else {
            // Wrap the entire response as result if it doesn't have standard structure
            response.result = result;
          }
          
          process.stdout.write(JSON.stringify(response) + '\n');
        } catch (e) {
          console.error('[Pierre Bridge] Failed to parse server response: ' + e.message);
          console.error('[Pierre Bridge] Raw response: ' + data);
          const errorResponse = {
            jsonrpc: '2.0',
            id: request.id,
            error: { 
              code: -32603, 
              message: 'Internal error: ' + e.message 
            }
          };
          process.stdout.write(JSON.stringify(errorResponse) + '\n');
        }
      });
    });
    
    req.on('error', (e) => {
      console.error('[Pierre Bridge] HTTP request failed: ' + e.message);
      const errorResponse = {
        jsonrpc: '2.0',
        id: request.id,
        error: { 
          code: -32603, 
          message: 'HTTP request failed: ' + e.message 
        }
      };
      process.stdout.write(JSON.stringify(errorResponse) + '\n');
    });
    
    req.write(postData);
    req.end();
  } catch (e) {
    console.error('[Pierre Bridge] Failed to process request: ' + e.message);
    const errorResponse = {
      jsonrpc: '2.0',
      id: null,
      error: { 
        code: -32700, 
        message: 'Parse error: ' + e.message 
      }
    };
    process.stdout.write(JSON.stringify(errorResponse) + '\n');
  }
});

process.stdin.on('end', () => {
  console.error('[Pierre Bridge] Shutting down');
  process.exit(0);
});

// Keep the process alive
process.on('SIGINT', () => {
  console.error('[Pierre Bridge] Received SIGINT');
  process.exit(0);
});

process.on('SIGTERM', () => {
  console.error('[Pierre Bridge] Received SIGTERM');
  process.exit(0);
});
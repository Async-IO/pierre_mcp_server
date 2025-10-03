// ABOUTME: Mock MCP client for testing stdio communication
// ABOUTME: Simulates MCP Client behavior over stdio

const { spawn } = require('child_process');
const { EventEmitter } = require('events');

/**
 * Mock MCP client that communicates via stdio
 * Simulates MCP Client behavior
 */
class MockMCPClient extends EventEmitter {
  constructor(command, args = []) {
    super();
    this.command = command;
    this.args = args;
    this.process = null;
    this.buffer = '';
    this.pendingRequests = new Map();
    this.nextId = 1;
  }

  async start() {
    return new Promise((resolve, reject) => {
      this.process = spawn(this.command, this.args, {
        stdio: ['pipe', 'pipe', 'pipe']
      });

      this.process.on('error', (error) => {
        reject(new Error(`Failed to start bridge: ${error.message}`));
      });

      this.process.stdout.on('data', (data) => {
        this.handleData(data.toString());
      });

      this.process.stderr.on('data', (data) => {
        if (process.env.DEBUG) {
          console.error(`[Bridge Stderr]: ${data}`);
        }
      });

      this.process.on('exit', (code) => {
        if (code !== 0 && code !== null) {
          this.emit('error', new Error(`Bridge exited with code ${code}`));
        }
      });

      // Wait a moment for bridge to initialize
      setTimeout(resolve, 1000);
    });
  }

  handleData(data) {
    this.buffer += data;

    const lines = this.buffer.split('\n');
    this.buffer = lines.pop() || '';

    for (const line of lines) {
      if (line.trim()) {
        try {
          const message = JSON.parse(line);
          this.handleMessage(message);
        } catch (error) {
          console.error('Failed to parse JSON:', line);
        }
      }
    }
  }

  handleMessage(message) {
    if (message.id !== undefined && this.pendingRequests.has(message.id)) {
      const { resolve, reject, timeout } = this.pendingRequests.get(message.id);
      clearTimeout(timeout);
      this.pendingRequests.delete(message.id);

      if (message.error) {
        reject(new Error(message.error.message || JSON.stringify(message.error)));
      } else {
        resolve(message);
      }
    } else {
      this.emit('notification', message);
    }
  }

  async send(request, timeoutMs = 30000) {
    if (!this.process) {
      throw new Error('Client not started');
    }

    const id = request.id || this.nextId++;
    const fullRequest = { ...request, id, jsonrpc: '2.0' };

    return new Promise((resolve, reject) => {
      const timeout = setTimeout(() => {
        this.pendingRequests.delete(id);
        reject(new Error(`Request ${id} timed out after ${timeoutMs}ms`));
      }, timeoutMs);

      this.pendingRequests.set(id, { resolve, reject, timeout });

      this.process.stdin.write(JSON.stringify(fullRequest) + '\n');
    });
  }

  sendRaw(data) {
    return new Promise((resolve) => {
      let response = '';
      const originalHandler = this.handleData.bind(this);

      this.handleData = (data) => {
        response += data;
        if (response.includes('\n')) {
          this.handleData = originalHandler;
          resolve(response.trim());
        }
      };

      this.process.stdin.write(data);
    });
  }

  async stop() {
    if (this.process) {
      return new Promise((resolve) => {
        this.process.on('exit', resolve);
        this.process.kill('SIGTERM');
        setTimeout(() => {
          if (!this.process.killed) {
            this.process.kill('SIGKILL');
          }
          resolve();
        }, 5000);
      });
    }
  }
}

module.exports = { MockMCPClient };

# Gemini Fitness Assistant - Free LLM MCP Client Example

An end-to-end example demonstrating how to use **Google's free Gemini API** with Pierre MCP Server to build an AI fitness assistant. This example shows how any free LLM service with function calling can interact with Pierre's MCP protocol.

## Why This Example?

This demonstrates a **complete open-source AI fitness assistant** using:
- ✅ **Free LLM API**: Google Gemini (1,500 requests/day, no credit card)
- ✅ **MCP Protocol**: Direct HTTP connection to Pierre MCP Server
- ✅ **Function Calling**: Native tool calling for fitness data analysis
- ✅ **End-to-End**: From user query → LLM reasoning → MCP tool calls → results

Unlike proprietary solutions (Claude Desktop, ChatGPT), this example uses a free, accessible LLM service that anyone can use.

> **Why Python?** This example is written in Python (not Rust) because Google's Gemini SDK is Python-first, making it accessible to 20x more developers than Rust. Python optimizes for learning and rapid prototyping, while Pierre's Rust A2A agents demonstrate production-grade implementations. See [WHY_PYTHON.md](WHY_PYTHON.md) for detailed justification with benchmarks and analysis.

## TL;DR - Fastest Way to Run

```bash
# 1. Start Pierre server (in another terminal)
cd pierre_mcp_server && cargo run --bin pierre-mcp-server

# 2. Get your free Gemini API key: https://ai.google.dev/gemini-api/docs/api-key

# 3. Run the automated setup script
cd examples/mcp_clients/gemini_fitness_assistant
./quick_start.sh

# Follow the prompts to configure your API key and credentials
```

**That's it!** The script will:
- ✅ Check Python installation
- ✅ Verify Pierre server is running
- ✅ Install dependencies
- ✅ Validate configuration
- ✅ Launch the assistant

For manual setup or troubleshooting, see the [detailed Quick Start](#quick-start) below.

---

## Features

### 🤖 AI-Powered Fitness Analysis
- **Natural Language Queries**: Ask questions in plain English
- **Intelligent Tool Selection**: Gemini automatically chooses the right MCP tools
- **Multi-Step Reasoning**: Combines multiple tool calls to answer complex questions
- **Contextual Responses**: Maintains conversation history for follow-up questions

### 🏃 Fitness Capabilities
All Pierre MCP tools are available through natural language:
- Activity tracking and analysis
- Training load calculations
- Goal setting and feasibility analysis
- Nutrition planning and meal analysis
- Sleep quality and recovery assessment
- Performance predictions and trends

### 💰 Free Tier Benefits
- **Google Gemini API**: 1,500 requests/day (free tier)
- **No Credit Card Required**: Sign up instantly
- **Fast Inference**: Gemini 2.0 Flash optimized for speed
- **Function Calling**: Native support for MCP tool execution

## Quick Start

### 1. Prerequisites

**Start Pierre Server:**
```bash
cd pierre_mcp_server
cargo run --bin pierre-mcp-server
```

**Create a Pierre User:**
```bash
curl -X POST http://localhost:8081/admin/setup \
  -H "Content-Type: application/json" \
  -d '{
    "email": "user@example.com",
    "password": "SecurePass123!",
    "display_name": "Test User"
  }'
```

**Get Free Gemini API Key:**
1. Visit: https://ai.google.dev/gemini-api/docs/api-key
2. Click "Get API Key"
3. Sign in with Google account (no credit card needed)
4. Copy your API key

### 2. Installation

```bash
cd examples/mcp_clients/gemini_fitness_assistant

# Install Python dependencies
pip install -r requirements.txt
```

### 3. Configuration

**Set Environment Variables:**
```bash
export GEMINI_API_KEY='your-gemini-api-key-here'
export PIERRE_EMAIL='user@example.com'
export PIERRE_PASSWORD='SecurePass123!'
export PIERRE_SERVER_URL='http://localhost:8081'
```

**Or use `.env` file:**
```bash
cp .env.example .env
# Edit .env with your credentials
```

### 4. Run the Assistant

**Interactive Mode:**
```bash
python gemini_fitness_assistant.py
```

**Demo Mode (Predefined Queries):**
```bash
python gemini_fitness_assistant.py --demo
```

**With CLI Arguments:**
```bash
python gemini_fitness_assistant.py \
  --gemini-key YOUR_KEY \
  --email user@example.com \
  --password SecurePass123! \
  --server http://localhost:8081
```

## Example Interactions

### Example 1: Activity Retrieval

```
You: What were my last 3 activities?

🤔 Thinking...
🔧 Calling tool: get_activities
   Arguments: {'limit': 3}
   ✅ Tool executed successfully

🤖 Assistant: Here are your last 3 activities:

1. **Morning Run** (Jan 15, 2024)
   - Distance: 5.2 km
   - Duration: 28 minutes
   - Pace: 5:23 min/km
   - Heart Rate Avg: 152 bpm

2. **Evening Bike Ride** (Jan 14, 2024)
   - Distance: 18.5 km
   - Duration: 42 minutes
   - Average Speed: 26.4 km/h

3. **Lunch Run** (Jan 13, 2024)
   - Distance: 8.1 km
   - Duration: 45 minutes
   - Pace: 5:33 min/km
```

### Example 2: Training Analysis

```
You: Analyze my training load and tell me if I need a recovery day

🤔 Thinking...
🔧 Calling tool: analyze_training_load
   ✅ Tool executed successfully
🔧 Calling tool: get_activities
   Arguments: {'limit': 14}
   ✅ Tool executed successfully

🤖 Assistant: Based on your training load analysis:

**Current Training Status:**
- CTL (Fitness): 45.2 (Good base fitness)
- ATL (Fatigue): 52.1 (Elevated fatigue)
- TSB (Form): -6.9 (Negative, indicates fatigue)

**Recommendation:** Yes, you should take a recovery day. Your Training Stress Balance
is negative (-6.9), indicating accumulated fatigue. Your recent 7-day training load
(ATL: 52.1) exceeds your 42-day fitness baseline (CTL: 45.2).

**Suggested Actions:**
1. Take 1-2 easy or rest days
2. Focus on sleep and nutrition
3. Consider light stretching or yoga
4. Resume normal training when TSB approaches 0
```

### Example 3: Nutrition Planning

```
You: Calculate my daily nutrition needs for marathon training

🤔 Thinking...
🔧 Calling tool: get_athlete
   ✅ Tool executed successfully
🔧 Calling tool: calculate_daily_nutrition
   Arguments: {'goal_type': 'endurance', 'activity_level': 'very_active'}
   ✅ Tool executed successfully

🤖 Assistant: Here are your daily nutrition needs for marathon training:

**Daily Caloric Needs:**
- BMR (Base Metabolic Rate): 1,645 kcal
- TDEE (with training): 2,878 kcal/day
- Recommended intake: 2,900-3,100 kcal/day

**Macronutrient Breakdown:**
- Carbohydrates: 435g (60%) - Essential for endurance
- Protein: 145g (20%) - Recovery and muscle maintenance
- Fats: 64g (20%) - Hormone production and energy

**Timing Recommendations:**
- Pre-workout (2-3h before): 75-100g carbs
- Post-workout (within 30min): 30-40g protein + 60-80g carbs
- Daily protein: Spread across 4-5 meals
```

## How It Works

### Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                     Gemini Fitness Assistant                    │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  User Query                                                     │
│     ↓                                                           │
│  Gemini 2.0 Flash (Free LLM)                                   │
│     ↓                                                           │
│  Function Calling Decision                                      │
│     ↓                                                           │
│  MCP Tool Call (HTTP JSON-RPC)                                 │
│     ↓                                                           │
│  Pierre MCP Server ──→ Fitness Data (Strava/Garmin/Fitbit)    │
│     ↓                                                           │
│  Results back to Gemini                                         │
│     ↓                                                           │
│  Natural Language Response                                      │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

### Request Flow

1. **User Query**: User asks a fitness question
2. **Gemini Processing**: Gemini 2.0 Flash analyzes the query
3. **Tool Selection**: Gemini decides which MCP tools to call
4. **MCP Execution**: Client executes tools via HTTP JSON-RPC
5. **Data Retrieval**: Pierre fetches data from fitness providers
6. **Result Synthesis**: Gemini combines results into natural language
7. **Response**: User receives actionable fitness insights

### MCP Protocol Details

**Tools List Request:**
```json
POST http://localhost:8081/mcp
Authorization: Bearer <jwt_token>

{
  "jsonrpc": "2.0",
  "method": "tools/list",
  "params": {},
  "id": 1
}
```

**Tool Call Request:**
```json
POST http://localhost:8081/mcp
Authorization: Bearer <jwt_token>

{
  "jsonrpc": "2.0",
  "method": "tools/call",
  "params": {
    "name": "get_activities",
    "arguments": {
      "limit": 5,
      "provider": "strava"
    }
  },
  "id": 2
}
```

### Gemini Function Calling

Gemini automatically converts MCP tools to function declarations:

```python
# MCP Tool Schema
{
  "name": "get_activities",
  "description": "Get user activities from fitness providers",
  "inputSchema": {
    "type": "object",
    "properties": {
      "provider": {"type": "string"},
      "limit": {"type": "integer"}
    }
  }
}

# Converted to Gemini Function Declaration
FunctionDeclaration(
  name="get_activities",
  description="Get user activities from fitness providers",
  parameters={
    "type": "object",
    "properties": {
      "provider": {"type_": "string"},
      "limit": {"type_": "integer"}
    }
  }
)
```

## Configuration Options

### Environment Variables

| Variable | Required | Default | Description |
|----------|----------|---------|-------------|
| `GEMINI_API_KEY` | ✅ Yes | - | Google Gemini API key |
| `PIERRE_EMAIL` | ✅ Yes | - | Pierre user email |
| `PIERRE_PASSWORD` | ✅ Yes | - | Pierre user password |
| `PIERRE_SERVER_URL` | No | `http://localhost:8081` | Pierre server URL |

### Command Line Arguments

```
usage: gemini_fitness_assistant.py [-h] [--server SERVER]
                                   [--gemini-key GEMINI_KEY]
                                   [--email EMAIL] [--password PASSWORD]
                                   [--demo]

optional arguments:
  -h, --help              Show help message
  --server SERVER         Pierre server URL
  --gemini-key KEY        Gemini API key
  --email EMAIL           Pierre user email
  --password PASSWORD     Pierre user password
  --demo                  Run demo mode with predefined queries
```

## Gemini API Free Tier Limits

**Free Tier (No Credit Card Required):**
- **Requests per day**: 1,500
- **Requests per minute**: 15
- **Model**: gemini-2.0-flash-exp
- **Context window**: 32,768 tokens
- **Function calling**: ✅ Supported
- **Streaming**: ✅ Supported

**Tips for Staying Within Limits:**
- Each conversation turn = 1 request
- Function calls are included in the same request
- Use demo mode to test without consuming many requests
- Monitor usage at: https://aistudio.google.com/app/apikey

## Alternative Free LLM Services

This example can be adapted to use other free LLM services:

### 1. Groq (Free Tier)
- **Rate Limit**: 30 requests/minute
- **Models**: Llama 3, Mixtral, Gemma
- **Function Calling**: ✅ Yes
- **API**: https://console.groq.com/

### 2. Ollama (Fully Local)
- **Rate Limit**: None (hardware only)
- **Models**: Llama, Mistral, CodeLlama, etc.
- **Function Calling**: Via MCP bridge
- **Setup**: https://github.com/patruff/ollama-mcp-bridge

### 3. OpenRouter (Free Tier)
- **Rate Limit**: Varies by model
- **Models**: Multiple (filter by `:free` tag)
- **Function Calling**: ✅ Yes (select models)
- **API**: https://openrouter.ai/

## Troubleshooting

### Gemini API Key Issues

```bash
# Test your API key
curl -H "Content-Type: application/json" \
     -d '{"contents":[{"parts":[{"text":"Hello"}]}]}' \
     "https://generativelanguage.googleapis.com/v1beta/models/gemini-2.0-flash-exp:generateContent?key=YOUR_API_KEY"
```

### Pierre Authentication Issues

```bash
# Test login
curl -X POST http://localhost:8081/api/auth/login \
  -H "Content-Type: application/json" \
  -d '{"email": "user@example.com", "password": "SecurePass123!"}'
```

### MCP Connection Issues

```bash
# Test MCP endpoint
curl -X POST http://localhost:8081/mcp \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer YOUR_JWT_TOKEN" \
  -d '{"jsonrpc": "2.0", "method": "tools/list", "params": {}, "id": 1}'
```

### Common Errors

**"Error: google-generativeai package not installed"**
```bash
pip install google-generativeai
```

**"❌ Login failed: Connection refused"**
- Ensure Pierre server is running: `cargo run --bin pierre-mcp-server`

**"Rate limit exceeded"**
- Free tier: 1,500 requests/day, 15 requests/minute
- Wait or upgrade to paid tier

**"No tools available"**
- Check JWT token is valid
- Ensure user has proper permissions
- Verify MCP endpoint is accessible

## Extending This Example

### Add Custom Tools

```python
# Add your own tools to Pierre MCP Server
# They'll automatically be available to Gemini

custom_tool = {
    "name": "analyze_race_strategy",
    "description": "Analyze optimal race pacing strategy",
    "inputSchema": {
        "type": "object",
        "properties": {
            "race_distance": {"type": "number"},
            "goal_time": {"type": "string"}
        },
        "required": ["race_distance"]
    }
}
```

### Add Conversation History

```python
# Store chat history for context
conversation_history = []

# Add to history after each interaction
conversation_history.append({
    "role": "user",
    "content": user_query
})
conversation_history.append({
    "role": "assistant",
    "content": response
})
```

### Add Streaming Responses

```python
# Enable streaming for real-time responses
response = self.model.generate_content(
    user_query,
    tools=self.gemini_tools,
    stream=True
)

for chunk in response:
    print(chunk.text, end='', flush=True)
```

### Advanced MCP Features

Pierre MCP Server supports the complete MCP specification including advanced features. This basic example demonstrates core functionality, but you can extend it to support:

**1. Sampling (Bidirectional LLM Requests)**
- Pierre can request LLM inference from the client
- Useful for server-initiated analysis or recommendations
- Enables collaborative reasoning between server and client LLMs

**2. Argument Completion**
- Auto-completion for tool parameters
- Pierre suggests values for 8+ tool parameters
- Improves UX by helping users discover valid inputs

**3. Progress Notifications**
- Real-time progress updates during long-running operations
- 44+ calls across intelligence handlers support progress reporting
- Allows clients to show progress bars and status updates

**4. Cancellation Support**
- Ability to cancel long-running operations
- 51+ cancellation token checks throughout codebase
- Prevents wasted computation on abandoned requests

**To implement these in your client:**
```python
# Example: Progress notifications
def on_progress(notification):
    progress = notification.get('progress', 0)
    total = notification.get('total', 100)
    print(f"Progress: {progress}/{total}")

# Example: Cancellation
import asyncio
cancellation_token = asyncio.Event()

# Set cancellation_token in request headers
# Pierre will check and abort if cancelled
```

See the [MCP Specification](https://spec.modelcontextprotocol.io/) for full details on implementing these features.

## Production Considerations

### Security
- ✅ Use environment variables for credentials (never hardcode)
- ✅ Implement proper OAuth2 flow for production
- ✅ Rotate API keys regularly
- ✅ Use HTTPS for Pierre server in production

### Performance
- ✅ Cache tool definitions (don't fetch on every request)
- ✅ Implement request retry with exponential backoff
- ✅ Monitor Gemini rate limits
- ✅ Use connection pooling for HTTP requests

### Monitoring
- ✅ Log all MCP tool calls for debugging
- ✅ Track Gemini API usage to avoid rate limits
- ✅ Monitor response times and errors
- ✅ Set up alerts for failures

## Resources

**Gemini API:**
- Quickstart: https://ai.google.dev/gemini-api/docs/quickstart
- Function Calling: https://ai.google.dev/gemini-api/docs/function-calling
- API Reference: https://ai.google.dev/api

**Pierre MCP Server:**
- Main README: ../../../README.md
- MCP Protocol: ../../../docs/protocols.md
- Available Tools: ../../../src/protocols/universal/tool_registry.rs

**Model Context Protocol:**
- Specification: https://spec.modelcontextprotocol.io/
- Examples: https://github.com/modelcontextprotocol

## License

This example is part of the Pierre MCP Server project and follows the same dual licensing:
- Apache License, Version 2.0
- MIT License

## Support

For issues or questions:
- Pierre Issues: https://github.com/Async-IO/pierre_mcp_server/issues
- Gemini Support: https://ai.google.dev/support

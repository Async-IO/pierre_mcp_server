# Gemini Fitness Assistant - Quick Start Guide

**Time to setup**: ~5 minutes
**Prerequisites**: Python 3.8+, Rust/Cargo installed

## Step 1: Start Pierre Server

Open a terminal and run:

```bash
cd pierre_mcp_server
cargo run --bin pierre-mcp-server
```

Keep this running. You should see:
```
Server started on 0.0.0.0:8081
```

---

## Step 2: Get Free Gemini API Key

1. Visit: https://ai.google.dev/gemini-api/docs/api-key
2. Click **"Get API Key"**
3. Sign in with your Google account (no credit card needed)
4. Click **"Create API key"**
5. Copy your API key (starts with `AIza...`)

**Free tier**: 1,500 requests/day, 15 requests/minute

---

## Step 3: Setup & Run

Open a **new terminal** and run:

```bash
cd examples/mcp_clients/gemini_fitness_assistant
./quick_start.sh
```

The script will guide you through:
- Installing Python dependencies
- Configuring your Gemini API key
- Setting up Pierre credentials
- Launching the assistant

---

## Step 4: Create Pierre Account

If you don't have a Pierre account yet, create one:

```bash
curl -X POST http://localhost:8081/admin/setup \
  -H "Content-Type: application/json" \
  -d '{
    "email": "your-email@example.com",
    "password": "YourSecurePassword123!",
    "display_name": "Your Name"
  }'
```

Use this email/password when the quick_start.sh script asks for credentials.

---

## Alternative: Manual Setup

If you prefer manual setup:

```bash
# Install dependencies
pip install -r requirements.txt

# Configure environment
export GEMINI_API_KEY='AIza...'
export PIERRE_EMAIL='your-email@example.com'
export PIERRE_PASSWORD='YourPassword123!'

# Run the assistant
python gemini_fitness_assistant.py
```

---

## What to Try

Once running, ask questions like:

```
You: What were my last 5 activities?
You: Analyze my training load for the past month
You: Calculate my daily nutrition needs for marathon training
You: What's my average pace for runs this week?
```

Type `quit` or `exit` to stop.

---

## Troubleshooting

**"❌ Pierre server is not running"**
- Make sure you started the server in Step 1
- Check it's accessible: `curl http://localhost:8081/health`

**"❌ Login failed"**
- Create a Pierre account using the curl command in Step 4
- Double-check your email and password

**"Error: google-generativeai package not installed"**
- Run: `pip install -r requirements.txt`

**"Rate limit exceeded"**
- Free tier: 1,500 requests/day, 15/minute
- Wait a minute or upgrade to paid tier

---

## Next Steps

- Connect a fitness provider (Strava/Garmin/Fitbit) at http://localhost:8081
- Explore more examples in [README.md](README.md)
- Try demo mode: `python gemini_fitness_assistant.py --demo`
- Check alternative free LLMs (Groq, Ollama) in the main README

---

## Support

- Full documentation: [README.md](README.md)
- Pierre issues: https://github.com/Async-IO/pierre_mcp_server/issues
- Gemini API docs: https://ai.google.dev/gemini-api/docs

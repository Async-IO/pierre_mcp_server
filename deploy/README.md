# Pierre MCP Server - GCP Deployment

Simple `gcloud` CLI-based deployment for Google Cloud Platform.

## Environment Configuration Mapping

| Local (.envrc)              | GCP Production                          |
|-----------------------------|-----------------------------------------|
| `DATABASE_URL`              | Cloud SQL + Secret Manager (DB_PASSWORD)|
| `PIERRE_MASTER_ENCRYPTION_KEY` | Secret Manager                       |
| `STRAVA_CLIENT_SECRET`      | Secret Manager                          |
| `OPENWEATHER_API_KEY`       | Secret Manager                          |
| `PIERRE_LLM_PROVIDER=local` | `PIERRE_LLM_PROVIDER=vertex`            |
| `VITE_*` (frontend)         | Build-time in `.env.production`         |
| Other env vars              | Cloud Run environment variables         |

**Key difference**: Secrets are stored in Secret Manager (not in config files).

## LLM Provider Configuration

In production, we use **Vertex AI** for LLM inference (set via `PIERRE_LLM_PROVIDER=vertex`).

| Provider | Use Case | Auth Method | Cost |
|----------|----------|-------------|------|
| `vertex` | **Production (recommended)** | Service account (automatic in Cloud Run) | Pay-per-use |
| `groq` | Alternative cloud | API key in Secret Manager | Pay-per-use |
| `gemini` | Development only | API key (20 req/day limit!) | Free tier limited |
| `local` | Local development | None (Ollama) | Free |

**Why Vertex AI?**
- No API key management (uses Cloud Run's service account)
- No rate limits (pay-per-use, ~$0.075/1M tokens)
- GCP-native billing (unified with other services)
- Same Gemini models as Google AI Studio

## Architecture

```
                    ┌─────────────────────┐
                    │   Cloud Run         │
                    │   (pierre-mcp)      │
                    │   scales 0-10       │
                    └─────────┬───────────┘
                              │
        ┌─────────────────────┼─────────────────────┐
        │               │               │           │
        ▼               ▼               ▼           ▼
┌─────────────┐   ┌─────────────┐   ┌─────────┐   ┌─────────────┐
│ Cloud SQL   │   │  Vertex AI  │   │ Secret  │   │  Artifact   │
│ PostgreSQL  │   │  (Gemini)   │   │ Manager │   │  Registry   │
│ (db-f1-micro)│   │             │   │         │   │             │
└─────────────┘   └─────────────┘   └─────────┘   └─────────────┘
```

**Estimated Cost**: ~$15-30/mo (Cloud SQL dominates; Cloud Run scales to zero)

## Prerequisites

1. [Google Cloud SDK](https://cloud.google.com/sdk/docs/install) installed
2. Authenticated: `gcloud auth login`
3. Project selected: `gcloud config set project pierrefitnessplatform`

## Quick Start

```bash
# 1. One-time setup (creates Cloud SQL, enables APIs, etc.)
./deploy/scripts/setup.sh

# 2. Add your OAuth credentials
./deploy/scripts/secrets.sh setup-oauth

# 3. Build the container image
./deploy/scripts/build.sh

# 4. Deploy to Cloud Run
./deploy/scripts/deploy.sh
```

## Scripts Reference

### `config.sh`
Configuration variables. Edit this to change region, instance sizes, etc.

### `scripts/setup.sh`
One-time project setup:
- Enables required GCP APIs
- Creates Artifact Registry repository
- Creates Cloud SQL PostgreSQL instance
- Creates database and user
- Stores credentials in Secret Manager
- Creates VPC connector for Cloud Run ↔ Cloud SQL

### `scripts/build.sh [tag]`
Builds container image using Cloud Build (remote build, no local Docker needed).
```bash
./scripts/build.sh           # Uses git SHA as tag
./scripts/build.sh v1.0.0    # Custom tag
```

### `scripts/deploy.sh [tag]`
Deploys to Cloud Run with all environment variables and secrets.
```bash
./scripts/deploy.sh          # Deploy :latest
./scripts/deploy.sh v1.0.0   # Deploy specific version
```

### `scripts/secrets.sh`
Manage secrets in Secret Manager.
```bash
./scripts/secrets.sh list                    # List all secrets
./scripts/secrets.sh add my-secret "value"   # Add/update secret
./scripts/secrets.sh get my-secret           # Get secret value
./scripts/secrets.sh setup-oauth             # Interactive OAuth setup
./scripts/secrets.sh add-oauth strava        # Add single provider
```

## Adding OAuth Providers

After setup, add your OAuth credentials:

```bash
# Interactive (recommended)
./scripts/secrets.sh setup-oauth

# Or manually
./scripts/secrets.sh add strava-client-id "YOUR_CLIENT_ID"
./scripts/secrets.sh add strava-client-secret "YOUR_CLIENT_SECRET"
./scripts/secrets.sh add strava-redirect-uri "https://YOUR_DOMAIN/api/oauth/callback/strava"
```

Then redeploy to pick up the new secrets:
```bash
./scripts/deploy.sh
```

## Updating the Deployment

```bash
# Build new image
./scripts/build.sh

# Deploy (uses :latest by default)
./scripts/deploy.sh

# Or deploy specific version
./scripts/build.sh v1.2.0
./scripts/deploy.sh v1.2.0
```

## Rollback

```bash
# List revisions
gcloud run revisions list --service=pierre-mcp-server --region=northamerica-northeast1

# Route traffic to previous revision
gcloud run services update-traffic pierre-mcp-server \
    --region=northamerica-northeast1 \
    --to-revisions=pierre-mcp-server-XXXXX=100
```

## Logs and Monitoring

```bash
# Stream logs
gcloud run services logs tail pierre-mcp-server --region=northamerica-northeast1

# View in console
open "https://console.cloud.google.com/run/detail/northamerica-northeast1/pierre-mcp-server/logs"
```

## Frontend Deployment

The frontend can be deployed to Firebase Hosting (free, recommended) or Cloud Storage.

### Firebase Hosting (Recommended)

```bash
# Install Firebase CLI if needed
npm install -g firebase-tools
firebase login

# Deploy frontend
./scripts/deploy-frontend.sh firebase
```

Frontend URL: `https://pierre-fitness-intelligence.web.app`

### Cloud Storage

```bash
./scripts/deploy-frontend.sh gcs
```

Frontend URL: `https://storage.googleapis.com/pierrefitnessplatform-frontend/index.html`

### How It Works

1. Script gets the backend URL from Cloud Run
2. Creates `frontend/.env.production` with:
   - `VITE_API_BASE_URL` pointing to Cloud Run
   - Firebase config for Google Sign-In
3. Builds frontend with `npm run build`
4. Deploys to chosen target
5. Updates backend's `FRONTEND_URL` environment variable

## SDK Configuration

The TypeScript SDK just needs the server URL:

```bash
# Users configure via environment variable
export PIERRE_SERVER_URL="https://pierre-mcp-server-xxxxx-nn.a.run.app"

# Or in their MCP client config (e.g., Claude Desktop)
{
  "mcpServers": {
    "pierre": {
      "command": "npx",
      "args": ["pierre-mcp-client"],
      "env": {
        "PIERRE_SERVER_URL": "https://pierre-mcp-server-xxxxx-nn.a.run.app"
      }
    }
  }
}
```

## Cleanup

To delete all resources:

```bash
# Delete Cloud Run service
gcloud run services delete pierre-mcp-server --region=northamerica-northeast1

# Delete Cloud SQL (remove deletion protection first)
gcloud sql instances patch pierre-postgres --no-deletion-protection
gcloud sql instances delete pierre-postgres

# Delete secrets
gcloud secrets delete pierre-mcp-server-db-password
gcloud secrets delete pierre-mcp-server-encryption-key

# Delete Artifact Registry
gcloud artifacts repositories delete pierre-images --location=northamerica-northeast1

# Delete VPC connector
gcloud compute networks vpc-access connectors delete pierre-vpc-connector --region=northamerica-northeast1
```

## CI/CD with GitHub Actions

Automated deployment on push to main.

### One-Time Setup

```bash
# Run from deploy/ directory
./scripts/setup-github-actions.sh
```

This creates:
1. **Workload Identity Pool** - Lets GitHub authenticate without JSON keys
2. **Service Account** - `github-actions-deployer@pierrefitnessplatform.iam.gserviceaccount.com`
3. **IAM Permissions** - Cloud Run, Cloud Build, Artifact Registry, etc.

### GitHub Secrets Required

Add these in: **Repository → Settings → Secrets and variables → Actions**

| Secret | Description |
|--------|-------------|
| `GCP_WORKLOAD_IDENTITY_PROVIDER` | Output from setup script |
| `GCP_SERVICE_ACCOUNT` | Output from setup script |
| `STRAVA_CLIENT_ID` | Your Strava app client ID |
| `FITBIT_CLIENT_ID` | Your Fitbit app client ID |
| `GARMIN_CLIENT_ID` | Your Garmin app client ID |
| `VITE_FIREBASE_API_KEY` | Firebase web API key |
| `VITE_FIREBASE_MESSAGING_SENDER_ID` | Firebase sender ID |
| `VITE_FIREBASE_APP_ID` | Firebase app ID |
| `VITE_FIREBASE_MEASUREMENT_ID` | Firebase analytics ID |
| `FIREBASE_SERVICE_ACCOUNT` | Firebase service account JSON |

**Note**: OAuth client *secrets* stay in GCP Secret Manager. Only client *IDs* go in GitHub.

### How It Works

```
Push to main
     │
     ▼
┌─────────────────┐    ┌─────────────────┐
│ Backend changed?│───▶│ Build container │───▶ Deploy to Cloud Run
└─────────────────┘    └─────────────────┘
     │
     ▼
┌──────────────────┐   ┌─────────────────┐
│ Frontend changed?│──▶│ npm build       │───▶ Deploy to Firebase
└──────────────────┘   └─────────────────┘
```

### Manual Trigger

Go to **Actions → Deploy to GCP → Run workflow** to deploy manually with options.

### Workflow Features

- **Smart detection**: Only deploys what changed (backend/frontend)
- **Workload Identity**: No service account keys stored in GitHub
- **Health check**: Verifies deployment before completing
- **Summary**: Shows deployment status in Actions tab

## Scaling Up Later

When you need more capacity:

### Upgrade Cloud SQL
```bash
gcloud sql instances patch pierre-postgres --tier=db-g1-small
```

### Add Redis (Memorystore)
```bash
gcloud redis instances create pierre-cache \
    --size=1 \
    --region=northamerica-northeast1 \
    --redis-version=redis_7_0

# Then add REDIS_URL to deploy.sh environment variables
```

### Enable HA for Cloud SQL
```bash
gcloud sql instances patch pierre-postgres --availability-type=REGIONAL
```

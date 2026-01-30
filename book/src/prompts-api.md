<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
<!-- Copyright (c) 2025 Pierre Fitness Intelligence -->

# Prompts API

Pierre provides REST APIs for managing AI chat prompt suggestions. Prompts are tenant-isolated and organized into three pillars: Activity, Nutrition, and Recovery.

## Overview

The prompts system consists of three components:
- **Prompt Categories**: Grouped suggestions displayed in the chat interface
- **Welcome Prompt**: Message shown to first-time users
- **System Prompt**: LLM instructions defining assistant behavior

## Authentication

All endpoints require JWT authentication via Bearer token or auth cookie:
```
Authorization: Bearer <jwt_token>
```

Admin endpoints additionally require `admin` or `super_admin` role.

## Public Endpoints

### Get Prompt Suggestions

Retrieves active prompt categories and welcome message for the authenticated user's tenant.

```http
GET /api/prompts/suggestions
Authorization: Bearer <jwt_token>
```

**Response** `200 OK`:
```json
{
  "categories": [
    {
      "category_key": "training",
      "category_title": "Training",
      "category_icon": "runner",
      "pillar": "activity",
      "prompts": [
        "Am I ready for a hard workout today?",
        "What's my predicted marathon time?"
      ]
    },
    {
      "category_key": "nutrition",
      "category_title": "Nutrition",
      "category_icon": "salad",
      "pillar": "nutrition",
      "prompts": [
        "How many calories should I eat today?",
        "What should I eat before my morning run?"
      ]
    },
    {
      "category_key": "recovery",
      "category_title": "Recovery",
      "category_icon": "sleep",
      "pillar": "recovery",
      "prompts": [
        "Do I need a rest day?",
        "Analyze my sleep quality"
      ]
    }
  ],
  "welcome_prompt": "Welcome to Pierre! I'm your fitness AI assistant. Connect your fitness tracker to get personalized insights.",
  "metadata": {
    "timestamp": "2025-01-07T12:00:00Z",
    "api_version": "1.0"
  }
}
```

## Admin Endpoints

All admin endpoints are prefixed with `/api/admin/prompts` and require admin role.

### List All Categories

Returns all prompt categories including inactive ones.

```http
GET /api/admin/prompts
Authorization: Bearer <admin_jwt_token>
```

**Response** `200 OK`:
```json
[
  {
    "id": "550e8400-e29b-41d4-a716-446655440000",
    "category_key": "training",
    "category_title": "Training",
    "category_icon": "runner",
    "pillar": "activity",
    "prompts": ["Am I ready for a hard workout today?"],
    "display_order": 0,
    "is_active": true
  }
]
```

### Create Category

Creates a new prompt category.

```http
POST /api/admin/prompts
Authorization: Bearer <admin_jwt_token>
Content-Type: application/json

{
  "category_key": "strength",
  "category_title": "Strength Training",
  "category_icon": "dumbbell",
  "pillar": "activity",
  "prompts": [
    "What's my estimated 1RM for bench press?",
    "Create a strength training plan"
  ],
  "display_order": 5
}
```

**Fields**:
| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `category_key` | string | Yes | Unique identifier within tenant |
| `category_title` | string | Yes | Display title |
| `category_icon` | string | Yes | Icon name (e.g., "runner", "salad", "sleep") |
| `pillar` | string | Yes | One of: `activity`, `nutrition`, `recovery` |
| `prompts` | string[] | Yes | List of prompt suggestions |
| `display_order` | integer | No | Sort order (default: 0) |

**Response** `201 Created`:
```json
{
  "id": "550e8400-e29b-41d4-a716-446655440001",
  "category_key": "strength",
  "category_title": "Strength Training",
  "category_icon": "dumbbell",
  "pillar": "activity",
  "prompts": ["What's my estimated 1RM for bench press?", "Create a strength training plan"],
  "display_order": 5,
  "is_active": true
}
```

### Get Category

Retrieves a specific category by ID.

```http
GET /api/admin/prompts/:id
Authorization: Bearer <admin_jwt_token>
```

**Response** `200 OK`: Same format as create response.

### Update Category

Updates an existing category. All fields are optional.

```http
PUT /api/admin/prompts/:id
Authorization: Bearer <admin_jwt_token>
Content-Type: application/json

{
  "category_title": "Strength & Power",
  "prompts": [
    "What's my estimated 1RM?",
    "Create a power building program",
    "How should I periodize my training?"
  ],
  "is_active": true
}
```

**Updatable Fields**:
| Field | Type | Description |
|-------|------|-------------|
| `category_title` | string | Display title |
| `category_icon` | string | Icon name |
| `pillar` | string | Pillar classification |
| `prompts` | string[] | Prompt suggestions |
| `display_order` | integer | Sort order |
| `is_active` | boolean | Visibility flag |

**Response** `200 OK`: Updated category object.

### Delete Category

Permanently deletes a category.

```http
DELETE /api/admin/prompts/:id
Authorization: Bearer <admin_jwt_token>
```

**Response** `204 No Content`

### Get Welcome Prompt

Retrieves the current welcome prompt.

```http
GET /api/admin/prompts/welcome
Authorization: Bearer <admin_jwt_token>
```

**Response** `200 OK`:
```json
{
  "prompt_text": "Welcome to Pierre! I'm your fitness AI assistant."
}
```

### Update Welcome Prompt

Updates the welcome prompt text.

```http
PUT /api/admin/prompts/welcome
Authorization: Bearer <admin_jwt_token>
Content-Type: application/json

{
  "prompt_text": "Hello! I'm Pierre, your personal fitness coach. How can I help you today?"
}
```

**Response** `200 OK`:
```json
{
  "prompt_text": "Hello! I'm Pierre, your personal fitness coach. How can I help you today?"
}
```

### Reset to Defaults

Resets all prompts (categories, welcome, system) to factory defaults.

```http
POST /api/admin/prompts/reset
Authorization: Bearer <admin_jwt_token>
```

**Response** `200 OK`:
```json
{
  "success": true
}
```

**Warning**: This operation deletes all custom categories and restores defaults. Prompt data is stored in the database and managed via the `PromptManager`. The system prompt template is at `src/llm/prompts/pierre_system.md`.

## Pillar Classification

Pillars provide visual organization and theming:

| Pillar | Color | Use For |
|--------|-------|---------|
| `activity` | Emerald (#10B981) | Training, workouts, performance |
| `nutrition` | Amber (#F59E0B) | Diet, calories, recipes, hydration |
| `recovery` | Indigo (#6366F1) | Sleep, rest days, stress, HRV |

## Error Responses

### 400 Bad Request
```json
{
  "error": {
    "code": "invalid_input",
    "message": "Invalid pillar: must be activity, nutrition, or recovery"
  }
}
```

### 401 Unauthorized
```json
{
  "error": {
    "code": "auth_required",
    "message": "Authentication required"
  }
}
```

### 403 Forbidden
```json
{
  "error": {
    "code": "permission_denied",
    "message": "Admin privileges required"
  }
}
```

### 404 Not Found
```json
{
  "error": {
    "code": "resource_not_found",
    "message": "Category not found"
  }
}
```

### 409 Conflict
```json
{
  "error": {
    "code": "resource_already_exists",
    "message": "Category with key 'training' already exists"
  }
}
```

## cURL Examples

### Get suggestions (user)
```bash
curl -X GET http://localhost:8081/api/prompts/suggestions \
  -H "Authorization: Bearer $JWT_TOKEN"
```

### Create category (admin)
```bash
curl -X POST http://localhost:8081/api/admin/prompts \
  -H "Authorization: Bearer $ADMIN_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "category_key": "cycling",
    "category_title": "Cycling",
    "category_icon": "bike",
    "pillar": "activity",
    "prompts": ["What is my FTP?", "Analyze my last ride"]
  }'
```

### Update welcome prompt (admin)
```bash
curl -X PUT http://localhost:8081/api/admin/prompts/welcome \
  -H "Authorization: Bearer $ADMIN_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"prompt_text": "Welcome! Ready to crush your fitness goals?"}'
```

### Reset to defaults (admin)
```bash
curl -X POST http://localhost:8081/api/admin/prompts/reset \
  -H "Authorization: Bearer $ADMIN_TOKEN"
```

## Tenant Isolation

All prompt data is tenant-scoped:
- Each tenant has independent prompt categories
- Category keys must be unique within a tenant (not globally)
- Admins can only modify prompts for their own tenant
- Super admins follow the same tenant isolation rules

## Related Documentation

- [Authentication](./authentication.md) - JWT token management
- [Configuration](./configuration.md) - Environment variables
- [LLM Providers](./llm-providers.md) - System prompt usage

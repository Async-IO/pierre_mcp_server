# appendix D: natural language to tool mapping

Quick reference mapping natural language prompts to Pierre MCP tools.

## connection & authentication

| User says... | Tool | Parameters |
|--------------|------|------------|
| "Connect to Pierre" | `connect_to_pierre` | None |
| "Link my Strava account" | `connect_provider` | `{"provider": "strava"}` |
| "Show my connections" | `get_connection_status` | None |
| "Disconnect from Fitbit" | `disconnect_provider` | `{"provider": "fitbit"}` |

## data access

| User says... | Tool | Parameters |
|--------------|------|------------|
| "Show my last 10 runs" | `get_activities` | `{"provider": "strava", "limit": 10}` |
| "Get my Strava profile" | `get_athlete` | `{"provider": "strava"}` |
| "What are my year-to-date stats?" | `get_stats` | `{"provider": "strava"}` |
| "Analyze activity 12345" | `get_activity_intelligence` | `{"activity_id": "12345", "provider": "strava"}` |

## performance analysis

| User says... | Tool | Parameters |
|--------------|------|------------|
| "Analyze my last workout" | `analyze_activity` | Activity data |
| "Am I getting faster?" | `analyze_performance_trends` | Historical activities |
| "Compare my last two rides" | `compare_activities` | Two activity IDs |
| "Find patterns in my training" | `detect_patterns` | Activities array |
| "What's my current fitness level?" | `calculate_fitness_score` | Activities + user profile |
| "Predict my marathon time" | `predict_performance` | Current fitness + race details |

## goals

| User says... | Tool | Parameters |
|--------------|------|------------|
| "Set a goal to run sub-20 5K" | `set_goal` | `{"type": "5K", "target_time": "00:20:00"}` |
| "How am I progressing?" | `track_progress` | Goal ID |
| "Suggest realistic goals" | `suggest_goals` | Current fitness level |
| "Can I run a 3-hour marathon?" | `analyze_goal_feasibility` | `{"goal_type": "marathon", "target_time": "03:00:00"}` |

## training recommendations

| User says... | Tool | Parameters |
|--------------|------|------------|
| "What should I work on?" | `generate_recommendations` | Performance analysis |
| "Am I overtraining?" | `analyze_training_load` | Recent activities |
| "Do I need a rest day?" | `suggest_rest_day` | Recovery metrics |

## nutrition

| User says... | Tool | Parameters |
|--------------|------|------------|
| "How many calories should I eat?" | `calculate_daily_nutrition` | User profile + activity level |
| "Search for banana nutrition" | `search_food` | `{"query": "banana"}` |
| "Show food details for ID 123" | `get_food_details` | `{"fdc_id": "123"}` |
| "Analyze this meal" | `analyze_meal_nutrition` | Array of foods with portions |
| "When should I eat carbs?" | `get_nutrient_timing` | Training schedule |

## sleep & recovery

| User says... | Tool | Parameters |
|--------------|------|------------|
| "How was my sleep?" | `analyze_sleep_quality` | Sleep session data |
| "What's my recovery score?" | `calculate_recovery_score` | Multi-factor recovery data |
| "Optimize my sleep schedule" | `optimize_sleep_schedule` | Sleep history |
| "Track my sleep trends" | `track_sleep_trends` | Sleep sessions over time |

## configuration

| User says... | Tool | Parameters |
|--------------|------|------------|
| "Update my FTP to 250W" | `update_user_configuration` | `{"ftp": 250}` |
| "Calculate my heart rate zones" | `calculate_personalized_zones` | User profile |
| "Show my configuration" | `get_user_configuration` | None |
| "What configuration profiles exist?" | `get_configuration_catalog` | None |

## prompt patterns

**Pattern 1: Temporal queries**
- "my last X..." → `limit: X, offset: 0`
- "this week..." → Filter by `start_date >= week_start`
- "in the past month..." → Filter by date range

**Pattern 2: Comparative queries**
- "compare A and B" → `compare_activities` with two IDs
- "better than..." → Fetch both, compare metrics

**Pattern 3: Trend queries**
- "am I improving?" → `analyze_performance_trends`
- "getting faster/slower?" → Trend analysis with slope

**Pattern 4: Predictive queries**
- "can I...?" → `analyze_goal_feasibility`
- "what if...?" → `predict_performance` with scenarios

## key takeaways

1. **Natural language**: AI assistants map user prompts to tool calls automatically.
2. **Temporal context**: "last 10", "this week", "past month" determine filters.
3. **Implicit parameters**: Provider often inferred from context or connection status.
4. **Tool chaining**: Complex queries combine multiple tools sequentially.
5. **Context awareness**: AI remembers previous queries for follow-up questions.

---

**End of Tutorial**

You've completed the comprehensive Pierre Fitness Platform tutorial! You now understand:
- **Part I**: Foundation (architecture, errors, config, DI)
- **Part II**: Authentication & Security (cryptography, JWT, multi-tenancy, middleware)
- **Part III**: MCP Protocol (JSON-RPC, request flow, transports, tool registry)
- **Part IV**: SDK & Type System (bridge architecture, type generation)
- **Part V**: OAuth, A2A & Providers (OAuth server/client, provider abstraction, A2A protocol)
- **Part VI**: Tools & Intelligence (45 tools, sports science algorithms, recovery, nutrition)
- **Part VII**: Testing & Deployment (synthetic data, design system, production deployment)

**Next Steps**:
1. Review CLAUDE.md for code standards
2. Explore the codebase using Appendix C as a map
3. Run the test suite to see synthetic data in action
4. Set up local development environment
5. Contribute improvements or new features

Happy coding! 🚀

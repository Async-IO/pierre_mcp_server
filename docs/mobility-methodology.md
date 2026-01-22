# Mobility Methodology

This document describes the scientific methodology and implementation of Pierre's mobility intelligence system, covering stretching exercises and yoga poses for athlete recovery and flexibility.

## Overview

Pierre's mobility system provides:

- **stretching exercises**: Static, dynamic, PNF, and ballistic stretching recommendations
- **yoga poses**: Recovery-focused yoga sequences with breath guidance
- **activity-specific**: Recommendations tailored to sport and muscle groups stressed
- **evidence-based**: Recommendations based on sports science research

**Target audience**: Developers, coaches, athletes, and users seeking deep understanding of Pierre's mobility intelligence.

## Table of Contents

- [Tool-to-Algorithm Mapping](#tool-to-algorithm-mapping)
- [1. Stretching Science](#1-stretching-science)
- [2. Yoga for Athletes](#2-yoga-for-athletes)
- [3. Activity-Muscle Mapping](#3-activity-muscle-mapping)
- [4. Sequence Generation](#4-sequence-generation)
- [5. MCP Tool Integration](#5-mcp-tool-integration)
- [6. Data Architecture](#6-data-architecture)
- [7. Scientific References](#7-scientific-references)

---

## Tool-to-Algorithm Mapping

This section provides a comprehensive mapping between MCP tools and their underlying algorithms, implementation files, and test coverage.

### Stretching Tools (3 tools)

| Tool Name | Algorithm/Intelligence | Implementation | Test File |
|-----------|----------------------|----------------|-----------|
| `list_stretching_exercises` | Database filtering by category/difficulty/muscle | `src/tools/implementations/mobility.rs:120-246` | `tests/mobility_tools_integration_test.rs` |
| `get_stretching_exercise` | Direct database retrieval with full details | `src/tools/implementations/mobility.rs:252-322` | `tests/mobility_tools_integration_test.rs` |
| `suggest_stretches_for_activity` | Activity-muscle mapping + warmup/cooldown categorization | `src/tools/implementations/mobility.rs:328-467` | `tests/mobility_e2e_test.rs` |

### Yoga Tools (3 tools)

| Tool Name | Algorithm/Intelligence | Implementation | Test File |
|-----------|----------------------|----------------|-----------|
| `list_yoga_poses` | Database filtering by category/difficulty/pose type/recovery context | `src/tools/implementations/mobility.rs:474-622` | `tests/mobility_tools_integration_test.rs` |
| `get_yoga_pose` | Direct database retrieval with full pose details | `src/tools/implementations/mobility.rs:628-706` | `tests/mobility_tools_integration_test.rs` |
| `suggest_yoga_sequence` | Balanced sequence generation with category diversity | `src/tools/implementations/mobility.rs:712-898` | `tests/mobility_e2e_test.rs` |

### Database Components

| Component | Purpose | Source File |
|-----------|---------|-------------|
| `MobilityManager` | CRUD operations for stretching/yoga data | `src/database/mobility.rs` |
| `StretchingExercise` | Stretching exercise model with muscles/instructions | `src/database/mobility.rs` |
| `YogaPose` | Yoga pose model with Sanskrit name/chakras/breath guidance | `src/database/mobility.rs` |
| `ActivityMuscleMapping` | Maps activities to primary/secondary muscles stressed | `src/database/mobility.rs` |

### Algorithm Reference Summary

| Algorithm | Purpose | Key Parameters |
|-----------|---------|----------------|
| **Activity-Muscle Mapping** | Links sport types to muscle groups | Primary muscles (high stress), secondary muscles (moderate stress) |
| **Warmup/Cooldown Selection** | Filters stretches by focus | Warmup → dynamic stretches, Cooldown → static stretches |
| **Sequence Balancing** | Creates varied yoga sequences | Category order: Standing → Balance → Seated → Supine → Twist |
| **Difficulty Filtering** | Ensures appropriate skill level | Beginner (1), Intermediate (2), Advanced (3) |
| **Duration Calculation** | Estimates sequence time | ~45s hold + 15s transition per pose |

---

## 1. Stretching Science

### Stretching Categories

Pierre supports four evidence-based stretching types:

| Category | Description | When to Use | Duration |
|----------|-------------|-------------|----------|
| **Static** | Hold position at end range | Post-workout cooldown | 30-60 seconds |
| **Dynamic** | Controlled movement through ROM | Pre-workout warmup | 10-15 reps |
| **PNF** | Contract-relax patterns | Flexibility training | 10-30 seconds |
| **Ballistic** | Bouncing movements (advanced) | Sport-specific prep | 10-15 reps |

### Scientific Basis

- **Static stretching**: Shown to reduce muscle stiffness and improve flexibility (Behm & Chaouachi, 2011)
- **Dynamic stretching**: Improves performance when used before activity (Opplert & Babault, 2018)
- **PNF (Proprioceptive Neuromuscular Facilitation)**: Most effective for flexibility gains (Sharman et al., 2006)

### Implementation

```rust
pub enum StretchingCategory {
    Static,      // Hold at end range
    Dynamic,     // Movement-based
    Pnf,         // Contract-relax
    Ballistic,   // Bouncing (advanced)
}
```

---

## 2. Yoga for Athletes

### Pose Categories

| Category | Description | Examples |
|----------|-------------|----------|
| **Standing** | Balance and leg strength | Warrior I/II, Tree Pose |
| **Seated** | Hip and hamstring flexibility | Seated Forward Fold, Pigeon |
| **Supine** | Back and hip openers | Happy Baby, Reclined Twist |
| **Prone** | Back extension and core | Cobra, Locust |
| **Inversion** | Blood flow and recovery | Legs Up Wall, Supported Headstand |
| **Balance** | Proprioception and focus | Eagle, Half Moon |
| **Twist** | Spinal mobility and digestion | Seated Twist, Revolved Triangle |

### Pose Types

| Type | Purpose | Recovery Context |
|------|---------|------------------|
| **Stretch** | Flexibility and ROM | Post-cardio, rest day |
| **Strength** | Muscle engagement | Active recovery |
| **Balance** | Proprioception | Injury prevention |
| **Relaxation** | Parasympathetic activation | Evening, stress relief |
| **Breathing** | Nervous system regulation | Any time |

### Recovery Contexts

Pierre recommends poses based on recovery needs:

- **post_cardio**: Focus on hip openers, hamstring stretches, quad releases
- **rest_day**: Full-body gentle flow with emphasis on relaxation
- **morning**: Energizing poses with breath work
- **evening**: Calming poses to prepare for sleep
- **stress_relief**: Focus on breathing and relaxation poses

---

## 3. Activity-Muscle Mapping

### Primary Muscle Groups by Activity

| Activity | Primary Muscles | Secondary Muscles |
|----------|-----------------|-------------------|
| **Running** | Quadriceps, hamstrings, calves, hip flexors | Glutes, core, ankles |
| **Cycling** | Quadriceps, hip flexors, calves | Hamstrings, glutes, lower back |
| **Swimming** | Shoulders, lats, triceps | Core, hip flexors, ankles |
| **Hiking** | Quadriceps, calves, glutes | Hamstrings, hip flexors, core |
| **Strength Training** | Varies by workout | Core, stabilizers |

### Implementation

```rust
pub struct ActivityMuscleMapping {
    pub activity_type: String,
    pub primary_muscles: Vec<String>,
    pub secondary_muscles: Vec<String>,
}
```

---

## 4. Sequence Generation

### Yoga Sequence Algorithm

The `suggest_yoga_sequence` tool uses a balanced approach:

1. **Filter by recovery context**: Get poses recommended for the purpose
2. **Filter by difficulty**: Ensure poses match user's skill level
3. **Apply focus area priority**: Sort by target muscle group if specified
4. **Balance categories**: Include variety (standing, seated, supine, twist)
5. **Calculate duration**: Target number of poses based on requested time
6. **Add relaxation**: Always end with Savasana or similar

### Category Order

```rust
const YOGA_CATEGORY_ORDER: [YogaCategory; 5] = [
    YogaCategory::Standing,
    YogaCategory::Balance,
    YogaCategory::Seated,
    YogaCategory::Supine,
    YogaCategory::Twist,
];
```

### Duration Estimation

- Average hold time: 45 seconds per pose
- Transition time: 15 seconds between poses
- Formula: `poses_count = duration_minutes.clamp(3, 12)`

---

## 5. MCP Tool Integration

### Tool Registration

All mobility tools are registered via `create_mobility_tools()`:

```rust
pub fn create_mobility_tools() -> Vec<Box<dyn McpTool>> {
    vec![
        Box::new(ListStretchingExercisesTool),
        Box::new(GetStretchingExerciseTool),
        Box::new(SuggestStretchesForActivityTool),
        Box::new(ListYogaPosesTool),
        Box::new(GetYogaPoseTool),
        Box::new(SuggestYogaSequenceTool),
    ]
}
```

### Tool Capabilities

All mobility tools require:
- `REQUIRES_AUTH`: User must be authenticated
- `READS_DATA`: Tools read from seeded database

---

## 6. Data Architecture

### Database Schema

Mobility data is seeded at database initialization:

- **stretching_exercises**: Pre-populated library of stretching exercises
- **yoga_poses**: Pre-populated library of yoga poses with Sanskrit names
- **activity_muscle_mappings**: Links activities to muscle groups

### Data Sources

- Stretching exercises: Based on ACSM guidelines and sports science literature
- Yoga poses: Traditional poses adapted for athletic recovery
- Activity mappings: Sport-specific biomechanical analysis

---

## 7. Scientific References

### Stretching Research

1. Behm, D.G., & Chaouachi, A. (2011). A review of the acute effects of static and dynamic stretching on performance. *European Journal of Applied Physiology*, 111(11), 2633-2651.

2. Opplert, J., & Babault, N. (2018). Acute effects of dynamic stretching on muscle flexibility and performance: an analysis of the current literature. *Sports Medicine*, 48(2), 299-325.

3. Sharman, M.J., Cresswell, A.G., & Riek, S. (2006). Proprioceptive neuromuscular facilitation stretching. *Sports Medicine*, 36(11), 929-939.

### Yoga for Athletes

4. Polsgrove, M.J., Eggleston, B.M., & Lockyer, R.J. (2016). Impact of 10-weeks of yoga practice on flexibility and balance of college athletes. *International Journal of Yoga*, 9(1), 27.

5. Woodyard, C. (2011). Exploring the therapeutic effects of yoga and its ability to increase quality of life. *International Journal of Yoga*, 4(2), 49.

### Recovery and Flexibility

6. Page, P. (2012). Current concepts in muscle stretching for exercise and rehabilitation. *International Journal of Sports Physical Therapy*, 7(1), 109.

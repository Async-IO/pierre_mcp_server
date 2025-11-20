# Test Intelligence Algorithms Skill

## Purpose
Validates sports science algorithms (VDOT, TSS, TRIMP, FTP, VO2max, Recovery, Nutrition) for mathematical correctness and physiological plausibility.

## CLAUDE.md Compliance
- ✅ Uses synthetic athlete data (no external dependencies)
- ✅ Deterministic tests with known outcomes
- ✅ Tests edge cases and error conditions
- ✅ Validates against research-based reference values

## Usage
Run this skill:
- After modifying algorithm implementations
- After changing algorithm configurations
- Before releases
- When validating sports science accuracy
- After updating physiological constants

## Prerequisites
- Rust toolchain
- Intelligence test fixtures in `tests/`

## Commands

### All Intelligence Tests
```bash
# Run all intelligence algorithm tests
cargo test intelligence -- --nocapture
```

### Basic Intelligence Tests
```bash
# Fundamental algorithm validation
cargo test --test intelligence_tools_basic_test -- --nocapture
```

### Advanced Intelligence Tests
```bash
# Complex scenarios and edge cases
cargo test --test intelligence_tools_advanced_test -- --nocapture
```

## Specific Algorithm Tests

### VDOT (Running Performance)
```bash
# Test VDOT calculation
cargo test test_vdot -- --nocapture

# Test race predictions
cargo test test_race_prediction -- --nocapture

# Test training paces
cargo test test_training_paces -- --nocapture
```

**Expected Values:**
```
Beginner (5K in 30:00):    VDOT ≈ 30-33
Intermediate (5K in 22:30): VDOT ≈ 45-48
Elite (5K in 16:00):       VDOT ≈ 67-70
```

### TSS/CTL/ATL/TSB (Training Load)
```bash
# Test Training Stress Score
cargo test test_tss -- --nocapture

# Test Chronic Training Load
cargo test test_ctl -- --nocapture

# Test Acute Training Load
cargo test test_atl -- --nocapture

# Test Training Stress Balance
cargo test test_tsb -- --nocapture
```

**Expected Values:**
```
Easy ride (IF=0.65, 60min):     TSS ≈ 40
Threshold (IF=1.00, 60min):     TSS = 100
VO2max interval (IF=1.15, 20min): TSS ≈ 44

Fresh athlete:   CTL=80, ATL=40 => TSB=+40
Fatigued:       CTL=70, ATL=100 => TSB=-30
```

### TRIMP (Training Impulse)
```bash
# Test TRIMP calculation
cargo test test_trimp -- --nocapture

# Test gender differences
cargo test test_trimp_gender -- --nocapture
```

**Expected Values:**
```
Male, 60min @ 75% HRmax:   TRIMP ≈ 90-110
Female, 60min @ 75% HRmax: TRIMP ≈ 80-95
```

### FTP (Functional Threshold Power)
```bash
# Test FTP estimation
cargo test test_ftp -- --nocapture

# Test 20-minute protocol
cargo test test_ftp_20min -- --nocapture

# Test 8-minute protocol
cargo test test_ftp_8min -- --nocapture

# Test ramp test
cargo test test_ftp_ramp -- --nocapture
```

**Expected Values:**
```
Beginner:     FTP ≈ 150W (20min test: 158W × 0.95)
Intermediate: FTP ≈ 250W (20min test: 263W × 0.95)
Elite:        FTP ≈ 380W (20min test: 400W × 0.95)
```

### VO2max (Aerobic Capacity)
```bash
# Test VO2max estimation
cargo test test_vo2max -- --nocapture

# Test Cooper 12-minute test
cargo test test_cooper_test -- --nocapture

# Test VDOT to VO2max conversion
cargo test test_vdot_to_vo2max -- --nocapture
```

**Expected Values:**
```
Beginner:     VO2max ≈ 35 ml/kg/min (Cooper: 1800m)
Intermediate: VO2max ≈ 49 ml/kg/min (Cooper: 2600m)
Elite:        VO2max ≈ 67 ml/kg/min (Cooper: 3400m)
```

### Recovery & Sleep
```bash
# Test recovery score calculation
cargo test test_recovery -- --nocapture

# Test sleep quality analysis
cargo test test_sleep_analysis -- --nocapture

# Test sleep stage scoring
cargo test test_sleep_stages -- --nocapture
```

**Expected Values:**
```
Well-rested: Recovery score > 70, Sleep quality > 80%
Adequate:    Recovery score 50-70, Sleep quality 60-80%
Poor:        Recovery score < 50, Sleep quality < 60%
```

### Nutrition
```bash
# Test BMR/TDEE calculation
cargo test test_nutrition -- --nocapture

# Test macronutrient distribution
cargo test test_macros -- --nocapture
```

**Expected Values:**
```
Adult male (70kg, 30yo):
  BMR ≈ 1700 kcal/day (Mifflin-St Jeor)
  TDEE (moderate activity) ≈ 2600 kcal/day
```

## Algorithm Configuration Testing

### Test Variant Switching
```bash
# Test VDOT algorithm variants
cargo test test_vdot_algorithm_config -- --nocapture

# Test MaxHR algorithm variants
cargo test test_maxhr_algorithm_config -- --nocapture

# Test TRIMP algorithm variants
cargo test test_trimp_algorithm_config -- --nocapture
```

### Test Configuration Loading
```bash
# Test environment variable configuration
export PIERRE_VDOT_ALGORITHM=daniels
cargo test test_algorithm_config_loading -- --nocapture
```

## Synthetic Test Data

### Beginner Athlete Profile
```rust
// 5K runner, novice
distance: 5000.0 meters
time: 1800.0 seconds (30:00)
expected_vdot: 30-33
max_hr: 190 bpm
age: 25
```

### Intermediate Athlete Profile
```rust
// Experienced cyclist
ftp: 250 watts
weight: 70 kg
power_to_weight: 3.57 W/kg
vo2max: 52 ml/kg/min
```

### Elite Athlete Profile
```rust
// Competitive triathlete
swim_threshold: 1:20/100m
bike_ftp: 380 watts
run_vdot: 67
vo2max: 72 ml/kg/min
```

## Edge Case Testing

### Zero Values
```bash
# Test handling of zero inputs
cargo test test_zero_inputs -- --nocapture
```

### Negative Values
```bash
# Test error handling for invalid inputs
cargo test test_negative_inputs -- --nocapture
```

### Extreme Outliers
```bash
# Test physiological limits
cargo test test_extreme_values -- --nocapture
```

**Physiological Bounds:**
```
MaxHR: 100-220 bpm
VO2max: 20-90 ml/kg/min
FTP: 50-600 watts
VDOT: 20-85
Recovery score: 0-100
```

## Test Output Analysis

### Success Example
```
test intelligence::algorithms::vdot::tests::test_daniels_formula ... ok
test intelligence::algorithms::vdot::tests::test_race_prediction ... ok
test intelligence::training_load::tests::test_tss_calculation ... ok
test intelligence::training_load::tests::test_ctl_buildup ... ok

test result: ok. 45 passed; 0 failed
```

### Detailed Output
```bash
# Show calculation details
cargo test test_vdot_calculation -- --nocapture

# Example output:
# Distance: 5000m, Time: 1350s (22:30)
# Velocity: 13.33 m/min
# VO2: 51.2 ml/kg/min
# VDOT: 47.3
# Predicted marathon: 3:12:45
```

## Validating Against Research

### Reference Values
```bash
# Compare against published tables
cargo test test_reference_values -- --nocapture

# Sources:
# - Daniels' Running Formula (VDOT tables)
# - Training and Racing with a Power Meter (TSS/FTP)
# - Bannister's TRIMP research
```

### Formula Verification
```bash
# Verify formula coefficients
rg "-4\.60|0\.182258|0\.000104" src/intelligence/algorithms/vdot.rs

# Check citations
rg "Reference:|Daniels|Coggan|Bannister" src/intelligence/algorithms/ -A 2
```

## Performance Testing

### Benchmark Algorithms
```bash
# Run algorithm benchmarks (if configured)
cargo bench --bench algorithm_benchmarks || echo "No benchmarks"

# Check performance regression
# VDOT calculation should be < 1µs
# TSS calculation should be < 5µs
```

## Success Criteria
- ✅ All algorithm tests pass
- ✅ Results match published reference values (±5%)
- ✅ Edge cases handled gracefully (errors returned)
- ✅ Physiological bounds validated
- ✅ Algorithm variants configurable
- ✅ No panic on invalid inputs
- ✅ Calculations deterministic (same input = same output)
- ✅ Performance acceptable (< 10µs per calculation)

## Common Issues

### Issue: VDOT calculation off by >5%
```bash
# Check formula coefficients
rg "-4\.60|0\.182258|0\.000104" src/intelligence/algorithms/vdot.rs

# Verify velocity calculation (m/min, not m/s!)
# velocity = (distance_meters / time_seconds) * 60.0
```

### Issue: TSS values seem incorrect
```bash
# Verify intensity factor calculation
# IF = NP / FTP  (normalized power / functional threshold power)
# TSS = (duration_hours * IF^2 * 100)

# Check for unit conversions (seconds vs hours)
```

### Issue: Recovery score always 100
```bash
# Check TSB component
# TSB = CTL - ATL
# Recovery score should incorporate sleep, HRV, etc.
```

## Troubleshooting

**Test fails with NaN:**
```rust
// Check for division by zero
if denominator.abs() < f64::EPSILON {
    return Err(AlgorithmError::DivisionByZero);
}
```

**Test fails with overflow:**
```rust
// Validate bounds before calculation
if value > MAX_PHYSIOLOGICAL_VALUE {
    return Err(AlgorithmError::ValueOutOfBounds);
}
```

## Related Files
- `tests/intelligence_tools_basic_test.rs` - Basic algorithm tests
- `tests/intelligence_tools_advanced_test.rs` - Advanced scenarios
- `src/intelligence/algorithms/` - Algorithm implementations
- `src/intelligence/physiological_constants.rs` - Bounds and constants
- `docs/intelligence-methodology.md` - Research documentation

## Related Skills
- `algorithm-validator.md` (agent) - Comprehensive algorithm validation
- `test-training-load-calculation.md` - Specific TSS/CTL/ATL testing
- `test-race-predictions.md` - VDOT race prediction validation

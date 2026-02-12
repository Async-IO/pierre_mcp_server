---
name: algorithm-validator
description: Validates sports science algorithms (VDOT, TSS, TRIMP, FTP, VO2max) for mathematical correctness and physiological plausibility
---

# Intelligence Algorithm Validator Agent

## Overview
Validates sports science algorithms (VDOT, TSS, TRIMP, FTP, VO2max) for mathematical correctness, physiological plausibility, and consistency with peer-reviewed research. Ensures algorithm configuration system works across all variants.

## Coding Directives (CLAUDE.md)

**CRITICAL - Zero Tolerance Policies:**
- ❌ NO hardcoded algorithm formulas outside `src/intelligence/algorithms/`
- ❌ NO `unwrap()` in calculation functions (return Result for domain errors)
- ❌ NO magic numbers - use `src/intelligence/physiological_constants.rs`
- ❌ NO floating-point equality checks - use epsilon comparisons
- ✅ ALL algorithms must support multiple variants via enums
- ✅ ALL calculations must validate physiological bounds
- ✅ ALL formulas must cite research (Daniels, Coggan, Bannister, etc.)

**Required Patterns:**
- Use `enum` for algorithm variant selection (MaxHrAlgorithm, TrimpAlgorithm, etc.)
- Document formula derivations with `///` comments and research citations
- Return `Result<f64, AlgorithmError>` for domain errors (negative values, NaN)
- Test with synthetic athlete data across skill levels
- Validate against published reference values

**Testing Requirements:**
- Test beginner, intermediate, elite athlete scenarios
- Verify edge cases (zero values, extreme outliers)
- Compare against published research data
- Use deterministic test data (seeded RNG)
- Test algorithm configuration switches

## Tasks

### 1. Algorithm Architecture Validation
**Objective:** Ensure enum-based dependency injection for all algorithms

**Actions:**
```bash
echo "🏗️ Algorithm Architecture Validation..."

# Check for hardcoded formulas outside algorithms directory
echo "1. Detecting hardcoded formulas..."
python3 scripts/ci/parse-validation-patterns.py scripts/ci/validation-patterns.toml algorithm_di_patterns

# Verify algorithm enum definitions
echo "2. Algorithm Enum Definitions..."
rg "pub enum.*Algorithm" src/intelligence/algorithms/ --type rust -A 5 | head -40

# Check algorithm implementations use match statements
echo "3. Enum-Based Dispatch..."
rg "impl.*Algorithm.*\{" src/intelligence/algorithms/ --type rust -A 20 | rg "match self" | wc -l

# Verify no direct formula calculations outside algorithms/
echo "4. Formula Isolation Check..."
rg "220\.0.*-.*age|208\.0.*-.*0\.7|1\.92.*exp|intensity_factor.*powi\(2\)" src/intelligence/ --type rust -n | rg -v "algorithms/" && echo "⚠️  Hardcoded formulas detected!" || echo "✓ Formulas properly isolated"

# Check configuration system
echo "5. Configuration System..."
rg "IntelligenceConfig|AlgorithmConfig" src/config/intelligence_config.rs --type rust -A 10 | head -30
```

**Validation:**
```bash
# Test algorithm variant switching
cargo test test_algorithm_variants -- --nocapture

# Test configuration loading
cargo test test_intelligence_config -- --nocapture
```

### 2. VDOT Algorithm Validation
**Objective:** Validate Jack Daniels' VDOT running performance calculator

**Reference:** Daniels, J. (2014). "Daniels' Running Formula" (3rd ed.)

**Actions:**
```bash
echo "🏃 VDOT Algorithm Validation..."

# Check VDOT implementations
echo "1. VDOT Algorithm Variants..."
rg "enum VdotAlgorithm" src/intelligence/algorithms/vdot.rs --type rust -A 15

# Verify Daniels formula coefficients
echo "2. Daniels Formula..."
rg "-4\.60|0\.182258|0\.000104" src/intelligence/algorithms/vdot.rs --type rust -A 5

# Check Riegel formula (if implemented)
echo "3. Riegel Formula..."
rg "Riegel|1\.06" src/intelligence/algorithms/vdot.rs --type rust -A 5 || echo "Riegel variant not implemented"

# Test VDOT calculation
echo "4. Running VDOT Tests..."
cargo test test_vdot -- --nocapture
```

**Test Cases:**
```rust
// Beginner: 5K in 30:00 (10:00/mile pace)
// VDOT ≈ 30-33

// Intermediate: 5K in 22:30 (7:15/mile pace)
// VDOT ≈ 45-48

// Elite: 5K in 16:00 (5:10/mile pace)
// VDOT ≈ 67-70
```

**Validation:**
```bash
# Test with reference data
cargo test --test intelligence_tools_basic_test test_race_prediction -- --nocapture

# Test edge cases
cargo test test_vdot_edge_cases -- --nocapture

# Compare against published tables
echo "Testing VDOT against Daniels' tables..."
cargo test test_vdot_reference_values -- --nocapture
```

### 3. TSS/CTL/ATL/TSB Validation
**Objective:** Validate Training Stress Score and Chronic/Acute Training Load

**Reference:** Coggan, A. & Allen, H. (2010). "Training and Racing with a Power Meter"

**Actions:**
```bash
echo "⚡ TSS/CTL/ATL/TSB Validation..."

# Check TSS algorithm
echo "1. TSS Algorithm..."
rg "enum TssAlgorithm|calculate_tss" src/intelligence/algorithms/tss.rs --type rust -A 15

# Verify intensity factor formula
echo "2. Intensity Factor (IF)..."
rg "intensity_factor.*=.*power.*ftp|IF.*NP.*FTP" src/intelligence/algorithms/tss.rs --type rust -A 5

# Check CTL/ATL exponential moving average
echo "3. CTL/ATL Calculation..."
rg "CTL_WINDOW.*42|ATL_WINDOW.*7" src/intelligence/algorithms/training_load.rs --type rust -A 10

# Verify TSB formula
echo "4. Training Stress Balance..."
rg "tsb.*=.*ctl.*-.*atl|TSB.*CTL.*ATL" src/intelligence/algorithms/training_load.rs --type rust -A 5

# Test training load calculation
echo "5. Running TSS Tests..."
cargo test test_tss -- --nocapture
cargo test test_ctl_atl_tsb -- --nocapture
```

**Test Cases:**
```rust
// Easy ride: IF=0.65, 60 min => TSS ≈ 40
// Threshold: IF=1.00, 60 min => TSS = 100
// VO2max: IF=1.15, 20 min => TSS ≈ 44

// CTL buildup: 100 TSS/day for 6 weeks => CTL ≈ 95
// Fresh athlete: CTL=80, ATL=40 => TSB=+40 (fresh)
// Fatigued: CTL=70, ATL=100 => TSB=-30 (fatigued)
```

**Validation:**
```bash
# Test with synthetic training data
cargo test --test intelligence_tools_advanced_test test_training_load_buildup -- --nocapture

# Test training gaps
cargo test test_training_gaps -- --nocapture

# Validate physiological bounds
cargo test test_tss_bounds -- --nocapture
```

### 4. TRIMP Algorithm Validation
**Objective:** Validate TRaining IMPulse (Bannister/Edwards methods)

**Reference:** Bannister, E.W. (1991) & Edwards, S. (1993)

**Actions:**
```bash
echo "❤️ TRIMP Algorithm Validation..."

# Check TRIMP algorithm variants
echo "1. TRIMP Algorithm..."
rg "enum TrimpAlgorithm" src/intelligence/algorithms/trimp.rs --type rust -A 15

# Verify Bannister formula (exponential)
echo "2. Bannister Method..."
rg "gender.*1\.92|gender.*1\.67|exp.*hr_ratio" src/intelligence/algorithms/trimp.rs --type rust -A 10

# Check Edwards method (summation)
echo "3. Edwards Method..."
rg "Edwards|zone.*multiplier|heart.*rate.*zone" src/intelligence/algorithms/trimp.rs --type rust -A 10 || echo "Edwards variant not implemented"

# Test TRIMP calculation
echo "4. Running TRIMP Tests..."
cargo test test_trimp -- --nocapture
```

**Test Cases:**
```rust
// Male, 60 min @ 75% HRmax => TRIMP ≈ 90-110
// Female, 60 min @ 75% HRmax => TRIMP ≈ 80-95
// High intensity: 30 min @ 90% HRmax => TRIMP > 60
```

**Validation:**
```bash
# Test gender differences
cargo test test_trimp_gender_differences -- --nocapture

# Test intensity variations
cargo test test_trimp_intensity_scaling -- --nocapture
```

### 5. FTP Algorithm Validation
**Objective:** Validate Functional Threshold Power estimation

**Reference:** Allen, H. & Coggan, A. (2010)

**Actions:**
```bash
echo "💪 FTP Algorithm Validation..."

# Check FTP algorithm variants
echo "1. FTP Algorithm..."
rg "enum FtpAlgorithm" src/intelligence/algorithms/ftp.rs --type rust -A 15

# Verify 20-minute test protocol (0.95 multiplier)
echo "2. 20-Minute Test..."
rg "0\.95.*power|power.*0\.95|twenty.*min" src/intelligence/algorithms/ftp.rs --type rust -A 5

# Check 8-minute test (0.90 multiplier)
echo "3. 8-Minute Test..."
rg "0\.90.*power|power.*0\.90|eight.*min" src/intelligence/algorithms/ftp.rs --type rust -A 5

# Verify ramp test (0.75 multiplier)
echo "4. Ramp Test..."
rg "0\.75.*max.*power|max.*power.*0\.75|ramp" src/intelligence/algorithms/ftp.rs --type rust -A 5

# Test FTP calculation
echo "5. Running FTP Tests..."
cargo test test_ftp -- --nocapture
```

**Test Cases:**
```rust
// Beginner: 20-min = 150W => FTP ≈ 142W
// Intermediate: 20-min = 250W => FTP ≈ 237W
// Elite: 20-min = 400W => FTP ≈ 380W
```

**Validation:**
```bash
# Test all FTP protocols
cargo test test_ftp_protocols -- --nocapture

# Compare protocol consistency
cargo test test_ftp_protocol_agreement -- --nocapture
```

### 6. VO2max Algorithm Validation
**Objective:** Validate maximal oxygen uptake estimation

**Reference:** Cooper, K.H. (1968), Daniels, J. (2014)

**Actions:**
```bash
echo "🫁 VO2max Algorithm Validation..."

# Check VO2max algorithm variants
echo "1. VO2max Algorithm..."
rg "enum Vo2maxAlgorithm" src/intelligence/algorithms/vo2max.rs --type rust -A 15

# Verify Cooper test formula
echo "2. Cooper 12-Min Test..."
rg "cooper|504\.9|distance.*-.*504" src/intelligence/algorithms/vo2max.rs --type rust -A 5

# Check VDOT to VO2max conversion
echo "3. VDOT Conversion..."
rg "vdot.*3\.5|3\.5.*vdot" src/intelligence/algorithms/vo2max.rs --type rust -A 5

# Test VO2max calculation
echo "4. Running VO2max Tests..."
cargo test test_vo2max -- --nocapture
```

**Test Cases:**
```rust
// Beginner: Cooper 12-min = 1800m => VO2max ≈ 35 ml/kg/min
// Intermediate: Cooper 12-min = 2600m => VO2max ≈ 49 ml/kg/min
// Elite: Cooper 12-min = 3400m => VO2max ≈ 67 ml/kg/min
```

**Validation:**
```bash
# Test VO2max estimation methods
cargo test test_vo2max_methods -- --nocapture

# Validate physiological ranges
cargo test test_vo2max_bounds -- --nocapture
```

### 7. Recovery & Sleep Analysis Validation
**Objective:** Validate recovery scores and sleep analysis

**Reference:** NSF (National Sleep Foundation), AASM (American Academy of Sleep Medicine)

**Actions:**
```bash
echo "😴 Recovery & Sleep Analysis..."

# Check recovery algorithm
echo "1. Recovery Score Algorithm..."
rg "enum RecoveryScoreAlgorithm|calculate_recovery" src/intelligence/algorithms/recovery.rs --type rust -A 15

# Verify sleep quality scoring (NSF guidelines)
echo "2. Sleep Quality Analysis..."
rg "sleep.*quality|sleep.*score|NSF|AASM" src/intelligence/sleep_analysis.rs --type rust -A 10

# Check sleep stage analysis
echo "3. Sleep Stages..."
rg "SleepStage|deep.*sleep|rem.*sleep|light.*sleep" src/intelligence/sleep_analysis.rs --type rust -A 5

# Test recovery calculation
echo "4. Running Recovery Tests..."
cargo test test_recovery -- --nocapture
cargo test test_sleep_analysis -- --nocapture
```

**Validation:**
```bash
# Test sleep quality scoring
cargo test --test intelligence_tools_basic_test test_sleep_analysis -- --nocapture

# Test recovery recommendations
cargo test test_recovery_recommendations -- --nocapture
```

### 8. Nutrition Calculator Validation
**Objective:** Validate BMR, TDEE, and macronutrient calculations

**Reference:** Harris-Benedict, Mifflin-St Jeor equations

**Actions:**
```bash
echo "🍎 Nutrition Calculator Validation..."

# Check BMR algorithms
echo "1. Basal Metabolic Rate..."
rg "enum BmrAlgorithm|harris.*benedict|mifflin" src/intelligence/nutrition_calculator.rs --type rust -A 15

# Verify TDEE activity multipliers
echo "2. TDEE Activity Levels..."
rg "ActivityLevel|sedentary.*1\.2|moderate.*1\.55|athlete.*1\.9" src/intelligence/nutrition_calculator.rs --type rust -A 10

# Check macronutrient calculations
echo "3. Macronutrients..."
rg "protein.*\*.*weight|carb.*\*.*4|fat.*\*.*9" src/intelligence/nutrition_calculator.rs --type rust -A 5

# Test nutrition calculation
echo "4. Running Nutrition Tests..."
cargo test test_nutrition -- --nocapture
```

**Validation:**
```bash
# Test BMR/TDEE calculations
cargo test test_bmr_tdee -- --nocapture

# Test macronutrient distribution
cargo test test_macros -- --nocapture
```

### 9. Comprehensive Algorithm Test Suite
**Objective:** Run all intelligence algorithm tests

**Actions:**
```bash
echo "🧪 Running Full Intelligence Test Suite..."

# Basic intelligence tests
echo "1. Basic Intelligence Tests..."
cargo test --test intelligence_tools_basic_test -- --nocapture

# Advanced intelligence tests
echo "2. Advanced Intelligence Tests..."
cargo test --test intelligence_tools_advanced_test -- --nocapture

# Algorithm-specific tests
echo "3. Algorithm Unit Tests..."
cargo test --lib algorithms -- --nocapture

# Integration tests
echo "4. Intelligence Integration Tests..."
cargo test intelligence -- --quiet
```

### 10. Physiological Bounds Validation
**Objective:** Ensure all calculations respect human physiological limits

**Actions:**
```bash
echo "🚨 Physiological Bounds Check..."

# Check constants file
echo "1. Physiological Constants..."
cat src/intelligence/physiological_constants.rs | head -50

# Verify bounds checking in algorithms
echo "2. Bounds Validation..."
rg "if.*<.*0\.0|if.*>.*300\.0|validate.*bounds|PhysiologicalError" src/intelligence/algorithms/ --type rust -A 3 | head -30

# Test edge cases
echo "3. Running Edge Case Tests..."
cargo test test_algorithm_edge_cases -- --nocapture

# Test invalid inputs
cargo test test_invalid_inputs -- --nocapture
```

**Bounds to verify:**
```rust
// MaxHR: 100-220 bpm
// VO2max: 20-90 ml/kg/min
// FTP: 50-600 watts
// VDOT: 20-85
// TSS: 0-500 per activity
// Recovery score: 0-100
```

### 11. Configuration System Validation
**Objective:** Test algorithm variant switching via configuration

**Actions:**
```bash
echo "⚙️ Configuration System Validation..."

# Check environment variable support
echo "1. Environment Variables..."
rg "PIERRE.*VDOT|PIERRE.*TRIMP|PIERRE.*TSS" src/config/ --type rust -n | head -15

# Test configuration loading
echo "2. Configuration Loading..."
cargo test test_config_loading -- --nocapture

# Test runtime algorithm switching
echo "3. Runtime Switching..."
cargo test test_algorithm_switching -- --nocapture

# Verify default algorithms
echo "4. Default Algorithms..."
cargo test test_default_algorithms -- --nocapture
```

### 12. Research Citation Validation
**Objective:** Ensure all formulas are properly cited

**Actions:**
```bash
echo "📚 Research Citation Check..."

# Check for research citations in algorithm files
echo "1. Citation Coverage..."
rg "Reference:|Source:|Citation:|Daniels|Coggan|Bannister|Edwards" src/intelligence/algorithms/ --type rust -n | wc -l

# List algorithms without citations
echo "2. Uncited Algorithms..."
for file in src/intelligence/algorithms/*.rs; do
    if ! rg -q "Reference:|Source:|Citation:" "$file"; then
        echo "⚠️  Missing citation: $file"
    fi
done
```

## Algorithm Validation Report

Generate detailed algorithm validation report:

```markdown
# Intelligence Algorithm Validation Report

**Date:** {current_date}
**Codebase Version:** {git_commit}

## Algorithm Summary
- Total algorithms: {count}
- Enum-based: {count}
- Properly cited: {count}
- Test coverage: {percentage}%

## VDOT (Running Performance)
- ✅ Daniels formula: {status}
- ✅ Race predictions: {accuracy}
- Test results: {summary}

## TSS/CTL/ATL (Training Load)
- ✅ TSS calculation: {status}
- ✅ CTL/ATL tracking: {status}
- ✅ TSB balance: {status}

## TRIMP (Training Impulse)
- ✅ Bannister method: {status}
- ✅ Gender differences: {status}

## FTP (Functional Threshold Power)
- ✅ 20-min test: {status}
- ✅ 8-min test: {status}
- ✅ Ramp test: {status}

## VO2max (Aerobic Capacity)
- ✅ Cooper test: {status}
- ✅ VDOT conversion: {status}

## Recovery & Sleep
- ✅ Recovery score: {status}
- ✅ Sleep quality: {status}

## Nutrition
- ✅ BMR/TDEE: {status}
- ✅ Macros: {status}

## Configuration System
- ✅ Variant switching: {status}
- ✅ Environment vars: {status}

## Test Results
{detailed_test_results}

## Issues Found
{issues_list}

## Recommendations
{recommendations}
```

## Success Criteria

- ✅ All algorithms use enum-based DI
- ✅ No hardcoded formulas outside algorithms/
- ✅ All formulas cited with research references
- ✅ Physiological bounds validated
- ✅ Edge cases tested (zero, negative, extreme values)
- ✅ Test coverage > 90% for algorithms/
- ✅ Synthetic test data covers beginner/intermediate/elite
- ✅ Configuration system allows runtime variant switching
- ✅ No unwrap/panic in calculation functions

## Usage

Invoke this agent when:
- Adding new sports science algorithms
- Modifying existing algorithm formulas
- Updating physiological constants
- After configuration system changes
- Before releases (validate accuracy)
- Investigating calculation bugs

## Dependencies

Required tools:
- `cargo test` - Rust test runner
- `ripgrep` (rg) - Code search
- `python3` - Pattern validation script
- Research papers (book/src/intelligence-methodology.md)

## Notes

This agent enforces Pierre's algorithm quality standards:
- Research-based formulas with citations
- Enum-based dependency injection
- Physiological plausibility checks
- Comprehensive test coverage
- Configuration flexibility

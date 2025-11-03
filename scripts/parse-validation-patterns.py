#!/usr/bin/env python3
"""
ABOUTME: Parse validation patterns from TOML configuration file
ABOUTME: Outputs shell-compatible variables for use in lint-and-test.sh
"""

import sys
import os
try:
    import tomllib
except ImportError:
    try:
        import tomli as tomllib
    except ImportError:
        import toml as tomllib

def escape_for_shell(text):
    """Escape text for safe use in shell regex patterns"""
    # For ripgrep, we need to be careful with escaping
    # Most patterns should work as-is, only escape the pipe character for shell
    return text

def build_pattern_list(patterns):
    """Build a pipe-separated regex pattern from a list of patterns"""
    if not patterns:
        return ""
    escaped_patterns = [escape_for_shell(pattern) for pattern in patterns]
    return "|".join(escaped_patterns)

def main():
    if len(sys.argv) != 2:
        print("Usage: parse-validation-patterns.py <toml-file>", file=sys.stderr)
        sys.exit(1)

    toml_file = sys.argv[1]

    if not os.path.exists(toml_file):
        print(f"Error: {toml_file} not found", file=sys.stderr)
        sys.exit(1)

    try:
        with open(toml_file, 'rb') as f:
            config = tomllib.load(f)
    except Exception as e:
        print(f"Error parsing {toml_file}: {e}", file=sys.stderr)
        sys.exit(1)

    # Extract pattern groups
    placeholder_patterns = config.get('placeholder_patterns', {})
    validation_config = config.get('validation_config', {})
    validation_thresholds = config.get('validation_thresholds', {})
    exclusions = config.get('exclusions', {})

    # Get critical, warning, threshold, and architectural groups
    critical_groups = validation_config.get('critical_groups', [])
    warning_groups = validation_config.get('warning_groups', [])
    threshold_groups = validation_config.get('threshold_groups', [])
    architectural_groups = validation_config.get('architectural_groups', [])

    # Build critical patterns (cause build failure)
    critical_patterns = []
    for group in critical_groups:
        if group in placeholder_patterns:
            critical_patterns.extend(placeholder_patterns[group])

    # Build warning patterns (logged but don't fail)
    warning_patterns = []
    for group in warning_groups:
        if group in placeholder_patterns:
            warning_patterns.extend(placeholder_patterns[group])

    # Build threshold patterns (count-based validation)
    threshold_patterns = []
    for group in threshold_groups:
        if group in placeholder_patterns:
            threshold_patterns.extend(placeholder_patterns[group])

    # Output shell variables
    print(f"CRITICAL_PATTERNS='{build_pattern_list(critical_patterns)}'")
    print(f"WARNING_PATTERNS='{build_pattern_list(warning_patterns)}'")
    print(f"THRESHOLD_PATTERNS='{build_pattern_list(threshold_patterns)}'")

    # Output individual group patterns for detailed reporting
    for group_name, patterns in placeholder_patterns.items():
        var_name = f"{group_name.upper()}_PATTERNS"
        print(f"{var_name}='{build_pattern_list(patterns)}'")

    # Output group classifications
    print(f"CRITICAL_GROUPS='{' '.join(critical_groups)}'")
    print(f"WARNING_GROUPS='{' '.join(warning_groups)}'")
    print(f"THRESHOLD_GROUPS='{' '.join(threshold_groups)}'")
    print(f"ARCHITECTURAL_GROUPS='{' '.join(architectural_groups)}'")

    # Output thresholds
    for threshold_name, threshold_value in validation_thresholds.items():
        var_name = f"{threshold_name.upper()}"
        print(f"{var_name}={threshold_value}")

    # Output exclusion patterns
    for exclusion_name, exclusion_patterns in exclusions.items():
        var_name = f"{exclusion_name.upper()}"
        # Convert glob patterns to space-separated string for bash arrays
        exclusion_list = " ".join(exclusion_patterns)
        print(f"{var_name}='{exclusion_list}'")

    # Output algorithm DI patterns
    algorithm_di_patterns = config.get('algorithm_di_patterns', {})
    migrated_algorithms = algorithm_di_patterns.get('migrated_algorithms', [])

    # Output list of migrated algorithms
    print(f"MIGRATED_ALGORITHMS='{' '.join(migrated_algorithms)}'")

    # For each algorithm, output its patterns and metadata
    for algo in migrated_algorithms:
        algo_config = algorithm_di_patterns.get(algo, {})
        algo_upper = algo.upper()

        # Output metadata
        print(f"ALGORITHM_{algo_upper}_NAME='{algo_config.get('name', algo)}'")
        print(f"ALGORITHM_{algo_upper}_ENUM='{algo_config.get('enum_name', '')}'")
        print(f"ALGORITHM_{algo_upper}_MODULE='{algo_config.get('module_path', '')}'")

        # Output exclude paths
        exclude_paths = algo_config.get('exclude_paths', [])
        if exclude_paths:
            print(f"ALGORITHM_{algo_upper}_EXCLUDES='{' '.join(exclude_paths)}'")

        # Output formula patterns
        formulas = algo_config.get('formulas', {})
        formula_patterns = list(formulas.values())
        if formula_patterns:
            combined_pattern = build_pattern_list(formula_patterns)
            print(f"ALGORITHM_{algo_upper}_PATTERNS='{combined_pattern}'")

if __name__ == "__main__":
    main()
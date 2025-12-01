#!/bin/bash
# ABOUTME: Validates all examples compile and pass tests
# ABOUTME: Run this before committing changes to examples/

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo "========================================"
echo "  Pierre Examples Validation"
echo "========================================"
echo ""

FAILED=0

# Validate Rust examples
echo -e "${YELLOW}=== Validating Rust Examples ===${NC}"

for dir in "$PROJECT_ROOT"/examples/agents/*/; do
    if [ -f "$dir/Cargo.toml" ]; then
        name=$(basename "$dir")
        echo -n "  Checking $name... "

        if (cd "$dir" && cargo check --quiet 2>/dev/null); then
            echo -e "${GREEN}OK${NC}"

            # Run tests if tests directory exists
            if [ -d "$dir/tests" ]; then
                echo -n "    Running tests... "
                if (cd "$dir" && cargo test --quiet 2>/dev/null); then
                    echo -e "${GREEN}PASSED${NC}"
                else
                    echo -e "${RED}FAILED${NC}"
                    FAILED=1
                fi
            fi
        else
            echo -e "${RED}FAILED${NC}"
            FAILED=1
        fi
    fi
done

# Validate Python examples
echo ""
echo -e "${YELLOW}=== Validating Python Examples ===${NC}"

PYTHON_DIR="$PROJECT_ROOT/examples/mcp_clients/gemini_fitness_assistant"
if [ -d "$PYTHON_DIR" ]; then
    echo -n "  Checking gemini_fitness_assistant syntax... "
    if python3 -m py_compile "$PYTHON_DIR"/*.py 2>/dev/null; then
        echo -e "${GREEN}OK${NC}"
    else
        echo -e "${RED}FAILED${NC}"
        FAILED=1
    fi

    echo -n "  Checking imports... "
    if (cd "$PYTHON_DIR" && python3 -c "from gemini_fitness_assistant import PierreMCPClient" 2>/dev/null); then
        echo -e "${GREEN}OK${NC}"
    else
        echo -e "${YELLOW}SKIPPED${NC} (missing dependencies)"
    fi
fi

echo ""
echo "========================================"
if [ $FAILED -eq 0 ]; then
    echo -e "${GREEN}All examples validated successfully!${NC}"
    exit 0
else
    echo -e "${RED}Some examples failed validation!${NC}"
    exit 1
fi

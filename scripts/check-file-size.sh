#!/bin/bash
# Check file sizes and warn about large files
#
# Thresholds based on Rust community best practices:
# - < 200 lines impl: Comfortable zone
# - 200-400 lines impl: Gray zone (watch for multiple concerns)
# - > 500 lines impl: Alarm zone (likely needs refactoring)

set -e

RED='\033[0;31m'
YELLOW='\033[1;33m'
GREEN='\033[0;32m'
CYAN='\033[0;36m'
NC='\033[0m'
BOLD='\033[1m'

GRAY_LIMIT=400
ALARM_LIMIT=500

echo -e "${BOLD}📏 File Size Health Check${NC}"
echo "================================================"
echo ""

alarm_count=0
gray_count=0

# Find all .rs files and check their sizes
for file in $(find src -name "*.rs" -type f | sort); do
    total=$(wc -l < "$file" | tr -d ' ')
    
    # Count test lines (from #[cfg(test)] to end)
    test_lines=$(awk '/#\[cfg\(test\)\]/{f=1} f{c++} END{print c+0}' "$file")
    
    # Count doc comment lines
    doc_lines=$(grep -c "^[[:space:]]*///" "$file" || true)
    if [ -z "$doc_lines" ]; then doc_lines=0; fi
    
    # Calculate implementation lines (simple subtraction)
    impl_lines=$((total - test_lines - doc_lines))
    if [ "$impl_lines" -lt 0 ]; then impl_lines=$total; fi
    
    if [ "$impl_lines" -gt "$ALARM_LIMIT" ]; then
        echo -e "${RED}🔴 ALARM${NC}: $file"
        echo -e "   Total: ${total} lines (impl: ~${impl_lines})"
        echo -e "   ${RED}→ Consider breaking into smaller modules${NC}"
        echo ""
        alarm_count=$((alarm_count + 1))
    elif [ "$impl_lines" -gt "$GRAY_LIMIT" ]; then
        echo -e "${YELLOW}🟡 WATCH${NC}: $file"
        echo -e "   Total: ${total} lines (impl: ~${impl_lines})"
        echo -e "   ${YELLOW}→ Ask: Does this file have a single clear responsibility?${NC}"
        echo ""
        gray_count=$((gray_count + 1))
    fi
done

echo "================================================"

if [ "$alarm_count" -gt 0 ]; then
    echo -e "${RED}${BOLD}$alarm_count file(s) need refactoring (>${ALARM_LIMIT} impl lines)${NC}"
    echo ""
    echo -e "${CYAN}📚 See docs/architecture/ for refactoring guidance:${NC}"
    echo "  • docs/architecture/layers.md - Layered architecture"
    echo "  • docs/architecture/directory.md - Proposed directory structure"
    echo "  • docs/architecture/review-senior.md - Module breakdown suggestions"
    echo ""
fi

if [ "$gray_count" -gt 0 ]; then
    echo -e "${YELLOW}${BOLD}$gray_count file(s) to watch (>${GRAY_LIMIT} impl lines)${NC}"
    echo ""
fi

if [ "$alarm_count" -eq 0 ] && [ "$gray_count" -eq 0 ]; then
    echo -e "${GREEN}✅ All files are within healthy size limits${NC}"
fi

# Only fail if there are alarm-level files
if [ "$alarm_count" -gt 0 ]; then
    exit 1
fi
exit 0

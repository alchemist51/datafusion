#!/bin/bash

# Script to run the aggregation + topk debug test with various options

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

echo -e "${BLUE}========================================${NC}"
echo -e "${BLUE}Aggregation + TopK Debug Test Runner${NC}"
echo -e "${BLUE}========================================${NC}"
echo ""

# Parse command line arguments
MODE=${1:-"debug"}

case $MODE in
  "debug")
    echo -e "${GREEN}Running with DEBUG level logging...${NC}"
    RUST_LOG=debug cargo test --test aggregation_topk_debug -- --nocapture
    ;;
  
  "trace")
    echo -e "${GREEN}Running with TRACE level logging (very verbose)...${NC}"
    RUST_LOG=trace cargo test --test aggregation_topk_debug -- --nocapture
    ;;
  
  "info")
    echo -e "${GREEN}Running with INFO level logging...${NC}"
    RUST_LOG=info cargo test --test aggregation_topk_debug -- --nocapture
    ;;
  
  "focused")
    echo -e "${GREEN}Running with focused logging (aggregates + topk only)...${NC}"
    RUST_LOG=datafusion_physical_plan::aggregates=debug,datafusion_physical_plan::topk=debug cargo test --test aggregation_topk_debug -- --nocapture
    ;;
  
  "memory")
    echo -e "${GREEN}Running with memory tracking focus...${NC}"
    RUST_LOG=datafusion_execution::memory_pool=debug,datafusion_physical_plan::aggregates=debug,datafusion_physical_plan::topk=debug cargo test --test aggregation_topk_debug -- --nocapture
    ;;
  
  "lldb")
    echo -e "${GREEN}Building test and preparing for lldb...${NC}"
    cargo test --test aggregation_topk_debug --no-run
    TEST_BINARY=$(find target/debug/deps -name 'aggregation_topk_debug-*' -type f -perm +111 | head -1)
    
    if [ -z "$TEST_BINARY" ]; then
      echo -e "${RED}Error: Could not find test binary${NC}"
      exit 1
    fi
    
    echo -e "${YELLOW}Test binary: $TEST_BINARY${NC}"
    echo -e "${YELLOW}Starting lldb...${NC}"
    echo ""
    echo -e "${BLUE}Suggested breakpoints:${NC}"
    echo "  b datafusion_physical_plan::aggregates::row_hash::GroupedHashAggregateStream::new"
    echo "  b datafusion_physical_plan::topk::TopK::insert_batch"
    echo "  b datafusion_physical_plan::topk::TopK::emit"
    echo ""
    echo -e "${BLUE}Then type 'r' to run${NC}"
    echo ""
    
    rust-lldb "$TEST_BINARY" -- test_aggregation_topk_interaction --nocapture
    ;;
  
  "help"|"-h"|"--help")
    echo "Usage: $0 [MODE]"
    echo ""
    echo "Modes:"
    echo "  debug    - Run with DEBUG level logging (default)"
    echo "  trace    - Run with TRACE level logging (very verbose)"
    echo "  info     - Run with INFO level logging (less verbose)"
    echo "  focused  - Run with focused logging (aggregates + topk only)"
    echo "  memory   - Run with memory tracking focus"
    echo "  lldb     - Build and run with lldb debugger"
    echo "  help     - Show this help message"
    echo ""
    echo "Examples:"
    echo "  $0              # Run with debug logging"
    echo "  $0 trace        # Run with trace logging"
    echo "  $0 focused      # Run with focused logging"
    echo "  $0 lldb         # Run with debugger"
    ;;
  
  *)
    echo -e "${RED}Unknown mode: $MODE${NC}"
    echo "Run '$0 help' for usage information"
    exit 1
    ;;
esac

echo ""
echo -e "${BLUE}========================================${NC}"
echo -e "${GREEN}Done!${NC}"
echo -e "${BLUE}========================================${NC}"

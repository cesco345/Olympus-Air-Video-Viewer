#!/bin/bash

# Script to run the Olympus Camera Terminal Control application
# with different options

# Define colors for better readability
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color

# Display banner
echo -e "${CYAN}╔════════════════════════════════════════╗${NC}"
echo -e "${CYAN}║     OLYMPUS CAMERA TERMINAL RUNNER     ║${NC}"
echo -e "${CYAN}╚════════════════════════════════════════╝${NC}"
echo ""

# Check command line arguments
if [ "$1" == "--debug" ] || [ "$1" == "-d" ]; then
    echo -e "${YELLOW}Running in debug mode with full logging...${NC}"
    cargo run -- --debug
elif [ "$1" == "--help" ] || [ "$1" == "-h" ]; then
    echo -e "Usage:"
    echo -e "  ${GREEN}./run.sh${NC}               - Run in normal mode"
    echo -e "  ${GREEN}./run.sh --debug${NC}       - Run with full debug logs"
    echo -e "  ${GREEN}./run.sh --help${NC}        - Show this help"
    echo ""
    echo -e "Camera IP is configured in src/main.rs"
    echo -e "Default is http://192.168.0.10"
else
    echo -e "${GREEN}Running in normal mode...${NC}"
    echo -e "Use ${YELLOW}./run.sh --debug${NC} for detailed logs"
    cargo run
fi
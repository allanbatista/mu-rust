#!/bin/bash
#
# convert_all_assets.sh
# =====================
#
# Wrapper script to convert all MU Online legacy assets to modern formats.
# This script orchestrates both texture and model conversion pipelines.
#
# Usage:
#   ./convert_all_assets.sh [OPTIONS]
#
# Options:
#   --textures-only     Only convert textures (skip models)
#   --models-only       Only convert models (skip textures)
#   --dry-run           Show what would be done without executing
#   --force             Force reconversion of all files
#   --verbose           Enable verbose logging
#   --help              Show this help message
#

set -e

# Default paths
#LEGACY_ROOT="${LEGACY_ROOT:-cpp/MuClient5.2/bin/Data}"
LEGACY_ROOT="${LEGACY_ROOT:-/home/allanbatista/Workspaces/Mu/MU_Red_1_20_61_Full/Data}"
OUTPUT_ROOT="${OUTPUT_ROOT:-rust/assets}"
BMD_CONVERTER="${BMD_CONVERTER:-bmd_converter.py}"

# Flags
CONVERT_TEXTURES=true
CONVERT_MODELS=true
DRY_RUN=""
FORCE=""
VERBOSE=""

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

function print_help() {
    echo "Usage: $0 [OPTIONS]"
    echo ""
    echo "Convert MU Online legacy assets to modern formats."
    echo ""
    echo "Options:"
    echo "  --textures-only     Only convert textures (skip models)"
    echo "  --models-only       Only convert models (skip textures)"
    echo "  --dry-run           Show what would be done without executing"
    echo "  --force             Force reconversion of all files"
    echo "  --verbose           Enable verbose logging"
    echo "  --help              Show this help message"
    echo ""
    echo "Environment Variables:"
    echo "  LEGACY_ROOT         Path to legacy assets (default: $LEGACY_ROOT)"
    echo "  OUTPUT_ROOT         Path to output directory (default: $OUTPUT_ROOT)"
    echo "  BMD_CONVERTER       Path to bmd_converter.py (default: $BMD_CONVERTER)"
    echo ""
}

function log_info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

function log_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

function log_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

function log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Parse arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        --textures-only)
            CONVERT_MODELS=false
            shift
            ;;
        --models-only)
            CONVERT_TEXTURES=false
            shift
            ;;
        --dry-run)
            DRY_RUN="--dry-run"
            shift
            ;;
        --force)
            FORCE="--force"
            shift
            ;;
        --verbose)
            VERBOSE="--verbose"
            shift
            ;;
        --help)
            print_help
            exit 0
            ;;
        *)
            log_error "Unknown option: $1"
            print_help
            exit 1
            ;;
    esac
done

# Validate paths
if [ ! -d "$LEGACY_ROOT" ]; then
    log_error "Legacy root directory not found: $LEGACY_ROOT"
    exit 1
fi

log_info "Starting asset conversion pipeline"
log_info "Legacy root: $LEGACY_ROOT"
log_info "Output root: $OUTPUT_ROOT"

START_TIME=$(date +%s)

# Step 1: Convert textures
if [ "$CONVERT_TEXTURES" = true ]; then
    log_info "Converting textures and auxiliary assets..."

    python3 assets_convert.py \
        --legacy-root "$LEGACY_ROOT" \
        --output-root "$OUTPUT_ROOT/data" \
        --skip-models \
        $DRY_RUN \
        $FORCE \
        $VERBOSE \
        --report "$OUTPUT_ROOT/reports/textures_report.json"

    if [ $? -eq 0 ]; then
        log_success "Texture conversion completed"
    else
        log_error "Texture conversion failed"
        exit 1
    fi
else
    log_info "Skipping texture conversion (--models-only specified)"
fi

# Step 2: Convert models (BMD → GLB)
if [ "$CONVERT_MODELS" = true ]; then
    log_info "Converting 3D models (BMD → GLB)..."

    python3 "$BMD_CONVERTER" \
        --bmd-root "$LEGACY_ROOT" \
        --output-root "$OUTPUT_ROOT/data" \
        --format glb \
        $DRY_RUN \
        $FORCE \
        $VERBOSE \
        --report "$OUTPUT_ROOT/reports/models_report.json"

    if [ $? -eq 0 ]; then
        log_success "Model conversion completed"
    else
        log_error "Model conversion failed"
        exit 1
    fi
else
    log_info "Skipping model conversion (--textures-only specified)"
fi

END_TIME=$(date +%s)
DURATION=$((END_TIME - START_TIME))

log_success "Asset conversion pipeline completed in ${DURATION}s"
log_info "Output directory: $OUTPUT_ROOT"

if [ -f "$OUTPUT_ROOT/reports/textures_report.json" ] || [ -f "$OUTPUT_ROOT/reports/models_report.json" ]; then
    log_info "Conversion reports available in: $OUTPUT_ROOT/reports/"
fi

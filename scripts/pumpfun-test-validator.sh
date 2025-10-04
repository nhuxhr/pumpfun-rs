#!/bin/bash

# Script to set up a local Solana test validator with Pump.fun and MPL Token Metadata programs.

# Exit on any error
set -e

# Check if the script is run from the correct directory
if [ ! -f "pumpfun-test-validator.sh" ]; then
  echo "Error: This script must be run from the pumpfun-rs/scripts directory."
  exit 1
fi

# Check if solana CLI tools are installed
if ! command -v solana >/dev/null 2>&1 || ! command -v solana-test-validator >/dev/null 2>&1; then
  echo "Error: Solana CLI tools are not installed or not in PATH. Please install them first."
  exit 1
fi

# Define configurable directories (default to ./programs and ./accounts)
PROGRAMS_DIR="${PROGRAMS_DIR:-./programs}"
ACCOUNTS_DIR="${ACCOUNTS_DIR:-./accounts}"

# Function to check and create a directory with write permissions
create_dir() {
  local dir="$1"
  if [ ! -d "$dir" ]; then
    if ! mkdir -p "$dir"; then
      echo "Error: Failed to create directory '$dir'. Check permissions."
      exit 1
    fi
    echo "Created directory: $dir"
  elif [ ! -w "$dir" ]; then
    echo "Error: Directory '$dir' exists but is not writable. Check permissions."
    exit 1
  fi
}

# Create required directories
create_dir "$PROGRAMS_DIR"
create_dir "$ACCOUNTS_DIR"

# Download Pump.fun program if it doesn't exist
PUMPFUN_SO="$PROGRAMS_DIR/pumpfun.so"
if [ ! -f "$PUMPFUN_SO" ]; then
  echo "Downloading Pump.fun program..."
  if ! solana program dump -u m 6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P "$PUMPFUN_SO"; then
    echo "Error: Failed to download Pump.fun program."
    exit 1
  fi
  echo "Downloaded Pump.fun program to $PUMPFUN_SO"
fi

# Download MPL Token Metadata program if it doesn't exist
MPL_TOKEN_METADATA_SO="$PROGRAMS_DIR/mpl-token-metadata.so"
if [ ! -f "$MPL_TOKEN_METADATA_SO" ]; then
  echo "Downloading MPL Token Metadata program..."
  if ! solana program dump -u m metaqbxxUerdq28cj1RbAWkYQm3ybzjb6a8bt518x1s "$MPL_TOKEN_METADATA_SO"; then
    echo "Error: Failed to download MPL Token Metadata program."
    exit 1
  fi
  echo "Downloaded MPL Token Metadata program to $MPL_TOKEN_METADATA_SO"
fi

# Download Pump.fun Global Account if it doesn't exist
PFG_ACCOUNT_JSON="$ACCOUNTS_DIR/4wTV1YmiEkRvAtNtsSGPtUrqRYQMe5SKy2uB4Jjaxnjf.json"
PFG_ACCOUNT_ADDRESS="4wTV1YmiEkRvAtNtsSGPtUrqRYQMe5SKy2uB4Jjaxnjf"
if [ ! -f "$PFG_ACCOUNT_JSON" ]; then
  echo "Downloading Pump.fun Global Account data..."
  if ! solana account -u m --output json --output-file "$PFG_ACCOUNT_JSON" "$PFG_ACCOUNT_ADDRESS"; then
    echo "Error: Failed to download account data."
    exit 1
  fi
  echo "Downloaded account data to $PFG_ACCOUNT_JSON"
fi

# Download Pump.fun Fee Config Account if it doesn't exist
PUMPFUN_FEE_CONFIG_JSON="$ACCOUNTS_DIR/8Wf5TiAheLUqBrKXeYg2JtAFFMWtKdG2BSFgqUcPVwTt.json"
PUMPFUN_FEE_CONFIG_ADDRESS="8Wf5TiAheLUqBrKXeYg2JtAFFMWtKdG2BSFgqUcPVwTt"
if [ ! -f "$PUMPFUN_FEE_CONFIG_JSON" ]; then
  echo "Downloading Pump.fun Fee Config Account data..."
  if ! solana account -u m --output json --output-file "$PUMPFUN_FEE_CONFIG_JSON" "$PUMPFUN_FEE_CONFIG_ADDRESS"; then
    echo "Error: Failed to download fee config account data."
    exit 1
  fi
  echo "Downloaded fee config account data to $PUMPFUN_FEE_CONFIG_JSON"
fi

# Download Pump.fun Fee Config program if it doesn't exist
PUMPFUN_FEE_CONFIG_SO="$PROGRAMS_DIR/pumpfun_fee_config.so"
if [ ! -f "$PUMPFUN_FEE_CONFIG_SO" ]; then
  echo "Downloading Pump.fun Fee Config program..."
  if ! solana program dump -u m pfeeUxB6jkeY1Hxd7CsFCAjcbHA9rWtchMGdZ6VojVZ "$PUMPFUN_FEE_CONFIG_SO"; then
    echo "Error: Failed to download Pump.fun Fee Config program."
    exit 1
  fi
  echo "Downloaded Pump.fun Fee Config program to $PUMPFUN_FEE_CONFIG_SO"
fi

# Build the validator command as an array for safety
COMMAND=(
  solana-test-validator
  -r
  --bpf-program "6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P" "$PUMPFUN_SO"
  --bpf-program "metaqbxxUerdq28cj1RbAWkYQm3ybzjb6a8bt518x1s" "$MPL_TOKEN_METADATA_SO"
  --bpf-program "pfeeUxB6jkeY1Hxd7CsFCAjcbHA9rWtchMGdZ6VojVZ" "$PUMPFUN_FEE_CONFIG_SO"
  --account "$PFG_ACCOUNT_ADDRESS" "$PFG_ACCOUNT_JSON"
  --account "$PUMPFUN_FEE_CONFIG_ADDRESS" "$PUMPFUN_FEE_CONFIG_JSON"
)

# Append any additional user-provided arguments
COMMAND+=("$@")

# Execute the command
echo "Starting Solana test validator..."
"${COMMAND[@]}"

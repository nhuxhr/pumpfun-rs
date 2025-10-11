# Pump.fun Solana Program SDK

<!--toc:start-->

- [Pump.fun Solana Program SDK](#pumpfun-solana-program-sdk)
  - [Getting Started](#getting-started)
  - [Usage](#usage)
    - [Local Development](#local-development)
  - [Features](#features)
  - [Feature Flags](#feature-flags)
  - [Architecture](#architecture)

<!--toc:end-->

This library provides a Rust interface for interacting with the Pump.fun Solana program.
Pump.fun is a Solana-based marketplace that enables users to create and distribute their own tokens, primarily memecoins.

## Getting Started
```sh
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source "$HOME/.cargo/env"
```
Add this crate to your project using cargo:

```sh
cargo add pumpfun
```

## Usage

The main entry point is the `PumpFun` struct which provides methods for interacting with the program:

> **Note:** The SDK automatically creates Associated Token Accounts (ATAs) when needed during buy transactions. No manual ATA creation is required.

### Local Development

For local development and testing, you can use the included test validator script:

```sh
# Navigate to the scripts directory
cd <path-to-pumpfun-rs-repo>/scripts

# Run the test validator
./pumpfun-test-validator.sh

# Or with custom options
./pumpfun-test-validator.sh --log
```

This script automatically downloads and configures a Solana test validator with the Pump.fun program and all required dependencies.

```rust,no_run
use pumpfun::{
    accounts::BondingCurveAccount,
    common::types::{Cluster, PriorityFee},
    utils::CreateTokenMetadata,
    PumpFun,
};
use solana_sdk::{
    commitment_config::CommitmentConfig,
    native_token::sol_to_lamports,
    native_token::LAMPORTS_PER_SOL,
    signature::{Keypair, Signature},
    signer::Signer,
};
use std::sync::Arc;

# tokio_test::block_on(async {
// Create a new PumpFun client
let payer = Arc::new(Keypair::new());
let client = PumpFun::new(
    payer.clone(),
    Cluster::localnet(CommitmentConfig::finalized(), PriorityFee::default()),
);

// Mint keypair
let mint = Keypair::new();

// Token metadata
let metadata = CreateTokenMetadata {
    name: "Lorem ipsum".to_string(),
    symbol: "LIP".to_string(),
    description: "Lorem ipsum dolor, sit amet consectetur adipisicing elit. Quam, nisi.".to_string(),
    file: "/path/to/image.png".to_string(),
    twitter: None,
    telegram: None,
    website: Some("https://example.com".to_string()),
};


// Track volume
let track_volume = Some(true);

// Optional priority fee to expedite transaction processing (e.g., 100 LAMPORTS per compute unit, equivalent to a 0.01 SOL priority fee)
let fee = Some(PriorityFee {
    unit_limit: Some(100_000),
    unit_price: Some(100_000_000),
});

// Create token with metadata
let signature = client.create(mint.insecure_clone(), metadata.clone(), fee).await.unwrap();
println!("Create signature: {}", signature);

// Create and buy tokens with metadata
let signature = client.create_and_buy(mint.insecure_clone(), metadata.clone(), sol_to_lamports(1f64), track_volume, None, fee).await.unwrap();
println!("Created and buy signature: {}", signature);

// Print the curve
let curve = client.get_bonding_curve_account(&mint.pubkey()).await.unwrap();
println!("Bonding curve: {:#?}", curve);

// Buy tokens (ATA will be created automatically if needed)
let signature = client.buy(mint.pubkey(), sol_to_lamports(1f64), track_volume, None, fee).await.unwrap();
println!("Buy signature: {}", signature);

// Sell tokens (sell all tokens)
let signature = client.sell(mint.pubkey(), None, None, fee).await.unwrap();
println!("Sell signature: {}", signature);

// Subscribe to real-time events with the stream feature
# #[cfg(feature = "stream")]
use pumpfun::common::stream::PumpFunEvent;

// Subscribe to Pump.fun events
# #[cfg(feature = "stream")]
let subscription = client.subscribe(None, None, |signature, event, error, _response| {
    match event {
        Some(PumpFunEvent::Create(create_event)) => {
            println!("New token created: {} ({})", create_event.name, create_event.symbol);
            println!("Mint address: {}", create_event.mint);
            println!("Created by: {}", create_event.creator);
        },
        Some(PumpFunEvent::Trade(trade_event)) => {
            let action = if trade_event.is_buy { "bought" } else { "sold" };
            println!(
                "User {} {} {} tokens for {} SOL",
                trade_event.user,
                action,
                trade_event.token_amount,
                trade_event.sol_amount as f64 / 1_000_000_000.0
            );
        },
        Some(event) => println!("Other event received: {:#?}", event),
        None => {
            if let Some(err) = error {
                eprintln!("Error parsing event in tx {}: {}", signature, err);
            }
        }
    }
}).await.unwrap();

// Keep subscription active as long as needed
// The subscription will automatically unsubscribe when dropped
# });
```

## Features

- Create new tokens with metadata and custom image
- Buy tokens using SOL with automatic ATA creation
- Sell tokens for SOL with slippage protection
- Query global and bonding curve state
- Calculate prices, fees and slippage
- Priority fee support for faster transactions
- IPFS metadata storage
- Real-time event subscriptions via WebSockets

## Feature Flags

The SDK provides several feature flags that can be enabled or disabled based on your requirements:

- `create-ata`: Enabled by default. Automatically creates Associated Token Accounts (ATAs) when needed during token purchases. Disable this if you want to handle ATA creation manually.

- `close-ata`: Enabled by default. Automatically closes unused ATAs after selling all tokens, helping to reclaim rent. Disable to keep ATAs open after selling.

- `versioned-tx`: Disabled by default. Enables support for Versioned Transactions with Address Lookup Tables (ALTs), which are useful for complex transactions with many accounts. When enabled, the SDK uses `VersionedTransaction` instead of legacy `Transaction`.

- `stream`: Disabled by default. Enables WebSocket-based event subscriptions for real-time monitoring of token creation, trading, and other on-chain events.

To customize feature flags in your `Cargo.toml`:

```toml
# Use default features (create-ata and close-ata enabled)
pumpfun = "4.6.0"

# Disable all default features
pumpfun = { version = "4.6.0", default-features = false }

# Custom selection of features
pumpfun = { version = "4.6.0", default-features = false, features = ["versioned-tx"] }

# Enable WebSocket event subscriptions
pumpfun = { version = "4.6.0", features = ["stream"] }
```

## Architecture

The SDK is organized into several modules:

- `accounts`: Account structs for deserializing on-chain state
- `common`: Common utility functions and types, including:
  - `stream`: WebSocket-based event subscription handling
- `constants`: Program constants like seeds and public keys
- `error`: Custom error types for error handling
- `instructions`: Transaction instruction builders
- `utils`: Helper functions and utilities

The main `PumpFun` struct provides high-level methods that abstract away the complexity of:

- Managing Program Derived Addresses (PDAs)
- Constructing and signing transactions
- Handling account lookups and deserialization
- Calculating prices, fees and slippage
- IPFS metadata uploads
- Priority fee configuration
- Associated Token Account management

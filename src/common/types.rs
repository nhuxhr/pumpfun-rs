//! Common types and utilities for the Pump.fun SDK
//!
//! This module provides common types and utilities that are used throughout the SDK, including:
//!
//! - Configuration structures for Solana clusters
//! - Priority fee settings for transactions
//! - Helper methods for connecting to different Solana networks
//!
//! These utilities help with configuring the connection to the Solana blockchain
//! and managing transaction parameters.

use serde::{Deserialize, Serialize};
use solana_sdk::commitment_config::CommitmentConfig;

/// Configuration for priority fee compute unit parameters
///
/// Priority fees allow transactions to be prioritized by validators based on
/// the fee paid per compute unit.
#[derive(Default, Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct PriorityFee {
    /// Maximum compute units that can be consumed by the transaction
    pub unit_limit: Option<u32>,
    /// Price in micro-lamports per compute unit
    pub unit_price: Option<u64>,
}

impl PriorityFee {
    /// Creates a new priority fee configuration
    ///
    /// # Arguments
    ///
    /// * `unit_limit` - Maximum compute units that can be consumed by the transaction
    /// * `unit_price` - Price in micro-lamports per compute unit
    ///
    /// # Returns
    ///
    /// A new `PriorityFee` instance with the specified configuration
    pub fn new(unit_limit: Option<u32>, unit_price: Option<u64>) -> Self {
        PriorityFee {
            unit_limit,
            unit_price,
        }
    }
}

/// RPC connection endpoints for a Solana cluster
///
/// # Fields
///
/// * `http` - HTTP endpoint URL for JSON RPC requests
/// * `ws` - WebSocket endpoint URL for subscription-based requests
#[derive(Debug, Clone)]
pub struct RpcEndpoint {
    pub http: String,
    pub ws: String,
}

impl RpcEndpoint {
    /// Creates a new RPC endpoint configuration
    ///
    /// # Arguments
    ///
    /// * `http` - HTTP endpoint URL for JSON RPC requests
    /// * `ws` - WebSocket endpoint URL for subscription-based requests
    ///
    /// # Returns
    ///
    /// A new `RpcEndpoint` instance with the specified endpoints
    pub fn new(http: String, ws: String) -> Self {
        RpcEndpoint { http, ws }
    }
}

/// Configuration for connecting to a Solana cluster
///
/// This structure contains all the necessary information to connect to a Solana cluster
/// and configure transaction parameters.
///
/// # Fields
///
/// * `rpc` - RPC endpoints for the cluster
/// * `commitment` - Commitment level for confirmations
/// * `priority_fee` - Priority fee configuration for transactions
#[derive(Debug, Clone)]
pub struct Cluster {
    pub rpc: RpcEndpoint,
    pub commitment: CommitmentConfig,
    pub priority_fee: PriorityFee,
}

impl Cluster {
    /// Creates a new cluster configuration with custom endpoints
    ///
    /// # Arguments
    ///
    /// * `http` - HTTP endpoint URL for the cluster
    /// * `ws` - WebSocket endpoint URL for the cluster
    /// * `commitment` - Commitment level for confirmations
    /// * `priority_fee` - Priority fee configuration for transactions
    ///
    /// # Returns
    ///
    /// A new `Cluster` instance with the specified configuration
    pub fn new(
        http: String,
        ws: String,
        commitment: CommitmentConfig,
        priority_fee: PriorityFee,
    ) -> Self {
        Self {
            rpc: RpcEndpoint { http, ws },
            commitment,
            priority_fee,
        }
    }

    /// Creates a configuration for the Solana mainnet-beta cluster
    ///
    /// # Arguments
    ///
    /// * `commitment` - Commitment level for confirmations
    /// * `priority_fee` - Priority fee configuration for transactions
    ///
    /// # Returns
    ///
    /// A `Cluster` instance configured for mainnet-beta
    pub fn mainnet(commitment: CommitmentConfig, priority_fee: PriorityFee) -> Self {
        Self::new(
            "https://api.mainnet-beta.solana.com".to_string(),
            "wss://api.mainnet-beta.solana.com".to_string(),
            commitment,
            priority_fee,
        )
    }

    /// Creates a configuration for the Solana devnet cluster
    ///
    /// # Arguments
    ///
    /// * `commitment` - Commitment level for confirmations
    /// * `priority_fee` - Priority fee configuration for transactions
    ///
    /// # Returns
    ///
    /// A `Cluster` instance configured for devnet
    pub fn devnet(commitment: CommitmentConfig, priority_fee: PriorityFee) -> Self {
        Self::new(
            "https://api.devnet.solana.com".to_string(),
            "wss://api.devnet.solana.com".to_string(),
            commitment,
            priority_fee,
        )
    }

    /// Creates a configuration for the Solana testnet cluster
    ///
    /// # Arguments
    ///
    /// * `commitment` - Commitment level for confirmations
    /// * `priority_fee` - Priority fee configuration for transactions
    ///
    /// # Returns
    ///
    /// A `Cluster` instance configured for testnet
    pub fn testnet(commitment: CommitmentConfig, priority_fee: PriorityFee) -> Self {
        Self::new(
            "https://api.testnet.solana.com".to_string(),
            "wss://api.testnet.solana.com".to_string(),
            commitment,
            priority_fee,
        )
    }

    /// Creates a configuration for a local Solana validator
    ///
    /// # Arguments
    ///
    /// * `commitment` - Commitment level for confirmations
    /// * `priority_fee` - Priority fee configuration for transactions
    ///
    /// # Returns
    ///
    /// A `Cluster` instance configured for a local validator with default ports
    pub fn localnet(commitment: CommitmentConfig, priority_fee: PriorityFee) -> Self {
        Self::new(
            "http://localhost:8899".to_string(),
            "ws://localhost:8900".to_string(),
            commitment,
            priority_fee,
        )
    }
}

#[cfg(feature = "amm")]
pub enum SwapInput {
    Base,
    Quote,
}

#[cfg(feature = "amm")]
pub enum SwapDirection {
    QuoteToBase,
    BaseToQuote,
}

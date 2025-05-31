//! Error types for the Pump.fun SDK.
//!
//! This module defines the `ClientError` enum, which encompasses various error types that can occur when interacting with the Pump.fun program.
//! It includes specific error cases for bonding curve operations, metadata uploads, Solana client errors, and more.
//!
//! The `ClientError` enum provides a comprehensive set of error types to help developers handle and debug issues that may arise during interactions with the Pump.fun program.
//!
//! # Error Types
//!
//! - `BondingCurveNotFound`: The bonding curve account was not found.
//! - `BondingCurveError`: An error occurred while interacting with the bonding curve.
//! - `BorshError`: An error occurred while serializing or deserializing data using Borsh.
//! - `SolanaClientError`: An error occurred while interacting with the Solana RPC client.
//! - `PubsubClientError`: An error occurred while interacting with the Solana Pubsub client.
//! - `UploadMetadataError`: An error occurred while uploading metadata to IPFS.
//! - `OtherError`: An error occurred that is not covered by the other error types.

#[derive(Debug)]
#[allow(clippy::large_enum_variant)]
pub enum ClientError {
    /// Bonding curve account was not found
    BondingCurveNotFound,
    /// Error related to bonding curve operations
    BondingCurveError(&'static str),
    /// Error deserializing data using Borsh
    BorshError(std::io::Error),
    /// Error from Solana RPC client
    SolanaClientError(solana_client::client_error::ClientError),
    /// Error from Solana Pubsub client
    #[cfg(feature = "stream")]
    PubsubClientError(solana_client::pubsub_client::PubsubClientError),
    /// Error uploading metadata
    UploadMetadataError(Box<dyn std::error::Error>),
    /// Other error
    OtherError(String),
}

impl std::fmt::Display for ClientError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::BondingCurveNotFound => write!(f, "Bonding curve not found"),
            Self::BondingCurveError(msg) => write!(f, "Bonding curve error: {}", msg),
            Self::BorshError(err) => write!(f, "Borsh serialization error: {}", err),
            Self::SolanaClientError(err) => write!(f, "Solana client error: {}", err),
            #[cfg(feature = "stream")]
            Self::PubsubClientError(err) => write!(f, "Solana pubsub client error: {}", err),
            Self::UploadMetadataError(err) => write!(f, "Metadata upload error: {}", err),
            Self::OtherError(msg) => write!(f, "Other error: {}", msg),
        }
    }
}

impl std::error::Error for ClientError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::BorshError(err) => Some(err),
            Self::SolanaClientError(err) => Some(err),
            #[cfg(feature = "stream")]
            Self::PubsubClientError(err) => Some(err),
            Self::UploadMetadataError(err) => Some(err.as_ref()),
            _ => None,
        }
    }
}

impl From<solana_client::client_error::ClientError> for ClientError {
    fn from(err: solana_client::client_error::ClientError) -> Self {
        Self::SolanaClientError(err)
    }
}

#[cfg(feature = "stream")]
impl From<solana_client::pubsub_client::PubsubClientError> for ClientError {
    fn from(err: solana_client::pubsub_client::PubsubClientError) -> Self {
        Self::PubsubClientError(err)
    }
}

//! Accounts for the Pump.fun Solana Program
//!
//! This module contains the definitions for the accounts used by the Pump.fun program.
//!
//! # Accounts
//!
//! - `BondingCurve`: Represents a bonding curve account.
//! - `Global`: Represents the global configuration account.

#[cfg(feature = "amm")]
pub mod amm;
mod bonding_curve;
mod global;

pub use bonding_curve::*;
pub use global::*;

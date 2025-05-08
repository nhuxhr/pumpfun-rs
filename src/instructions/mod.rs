//! Instructions for the Pump.fun Solana Program
//!
//! This module contains the definitions for instructions that can be sent to the Pump.fun program.
//!
//! # Instructions
//!
//! - `Create`: Creates a new token with an associated bonding curve.
//! - `Buy`: Buys tokens from a bonding curve by providing SOL.
//! - `Sell`: Sells tokens back to the bonding curve in exchange for SOL.

#[cfg(feature = "amm")]
mod amm;
mod buy;
mod create;
mod sell;

pub use buy::*;
pub use create::*;
pub use sell::*;

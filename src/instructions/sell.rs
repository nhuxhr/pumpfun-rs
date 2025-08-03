//! Instruction for selling tokens back to bonding curves
//!
//! This module provides the functionality to sell tokens back to bonding curves.
//! It includes the instruction data structure and helper function to build the Solana instruction.

use crate::{constants, PumpFun};
use borsh::{BorshDeserialize, BorshSerialize};
use solana_sdk::{
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
    signature::Keypair,
    signer::Signer,
};
use spl_associated_token_account::get_associated_token_address;

/// Instruction data for selling tokens back to a bonding curve
///
/// # Fields
///
/// * `amount` - Amount of tokens to sell (in token smallest units)
/// * `min_sol_output` - Minimum acceptable SOL received for the sale (slippage protection)
#[derive(BorshSerialize, BorshDeserialize, Clone)]
pub struct Sell {
    pub amount: u64,
    pub min_sol_output: u64,
}

impl Sell {
    /// Instruction discriminator used to identify this instruction
    pub const DISCRIMINATOR: [u8; 8] = [51, 230, 133, 164, 1, 127, 131, 173];

    /// Serializes the instruction data with the appropriate discriminator
    ///
    /// # Returns
    ///
    /// Byte vector containing the serialized instruction data
    pub fn data(&self) -> Vec<u8> {
        let mut data = Vec::with_capacity(256);
        data.extend_from_slice(&Self::DISCRIMINATOR);
        self.serialize(&mut data).unwrap();
        data
    }
}

/// Creates an instruction to sell tokens back to a bonding curve
///
/// Sells tokens back to the bonding curve in exchange for SOL. The amount of SOL received
/// is calculated based on the bonding curve formula. A portion of the SOL is taken as
/// a fee and sent to the fee recipient account. The price decreases as more tokens are
/// sold according to the bonding curve function.
///
/// # Arguments
///
/// * `payer` - Keypair that owns the tokens to sell
/// * `mint` - Public key of the token mint to sell
/// * `fee_recipient` - Public key of the account that will receive the transaction fee
/// * `creator` - Public key of the token's creator
/// * `args` - Sell instruction data containing token amount and minimum acceptable SOL output
///
/// # Returns
///
/// Returns a Solana instruction that when executed will sell tokens to the bonding curve
///
/// # Account Requirements
///
/// The instruction requires the following accounts in this order:
/// 1. Global configuration PDA (readonly)
/// 2. Fee recipient account (writable)
/// 3. Token mint account (readonly)
/// 4. Bonding curve PDA (writable)
/// 5. Bonding curve token account (writable)
/// 6. Seller's token account (writable)
/// 7. Payer account (signer, writable)
/// 8. System program (readonly)
/// 9. Creator vault (writable)
/// 10. Token program (readonly)
/// 11. Event authority (readonly)
/// 12. Pump.fun program ID (readonly)
pub fn sell(
    payer: &Keypair,
    mint: &Pubkey,
    fee_recipient: &Pubkey,
    creator: &Pubkey,
    args: Sell,
) -> Instruction {
    let bonding_curve: Pubkey = PumpFun::get_bonding_curve_pda(mint).unwrap();
    let creator_vault: Pubkey = PumpFun::get_creator_vault_pda(creator).unwrap();
    Instruction::new_with_bytes(
        constants::accounts::PUMPFUN,
        &args.data(),
        vec![
            AccountMeta::new_readonly(PumpFun::get_global_pda(), false),
            AccountMeta::new(*fee_recipient, false),
            AccountMeta::new_readonly(*mint, false),
            AccountMeta::new(bonding_curve, false),
            AccountMeta::new(get_associated_token_address(&bonding_curve, mint), false),
            AccountMeta::new(get_associated_token_address(&payer.pubkey(), mint), false),
            AccountMeta::new(payer.pubkey(), true),
            AccountMeta::new_readonly(constants::accounts::SYSTEM_PROGRAM, false),
            AccountMeta::new(creator_vault, false),
            AccountMeta::new_readonly(constants::accounts::TOKEN_PROGRAM, false),
            AccountMeta::new_readonly(constants::accounts::EVENT_AUTHORITY, false),
            AccountMeta::new_readonly(constants::accounts::PUMPFUN, false),
            AccountMeta::new(constants::accounts::GLOBAL_VOLUME_ACCUMULATOR, false),
            AccountMeta::new(
                PumpFun::get_user_volume_accumulator_pda(&payer.pubkey()),
                false,
            ),
        ],
    )
}

//! Global account for the Pump.fun Solana Program
//!
//! This module contains the definition for the global configuration account.
//!
//! # Global Account
//!
//! The global account is used to store the global configuration for the Pump.fun program.
//!
//! # Fields
//!
//! - `discriminator`: Unique identifier for the global account
//! - `initialized`: Whether the global account has been initialized
//! - `authority`: Authority pubkey that can modify settings
//! - `fee_recipient`: Account that receives fees
//! - `initial_virtual_token_reserves`: Initial virtual token reserves for price calculations
//! - `initial_virtual_sol_reserves`: Initial virtual SOL reserves for price calculations
//! - `initial_real_token_reserves`: Initial actual token reserves available for trading
//! - `token_total_supply`: Total supply of tokens
//! - `fee_basis_points`: Fee in basis points (1/100th of a percent)
//! - `withdraw_authority`: Authority that can withdraw funds
//! - `enable_migrate`: Flag to enable pool migration
//! - `pool_migration_fee`: Fee for migrating pools
//! - `creator_fee`: Fee for creators
//! - `fee_recipients`: Array of public keys for fee recipients
//!
//! # Methods
//!
//! - `new`: Creates a new global account instance
//! - `get_initial_buy_price`: Calculates the initial amount of tokens received for a given SOL amount

use borsh::{BorshDeserialize, BorshSerialize};
use solana_sdk::pubkey::Pubkey;

/// Represents the global configuration account for token pricing and fees
#[derive(Debug, Clone, BorshSerialize, BorshDeserialize)]
pub struct GlobalAccount {
    /// Unique identifier for the global account
    pub discriminator: u64,
    /// Whether the global account has been initialized
    pub initialized: bool,
    /// Authority that can modify global settings
    pub authority: Pubkey,
    /// Account that receives fees
    pub fee_recipient: Pubkey,
    /// Initial virtual token reserves for price calculations
    pub initial_virtual_token_reserves: u64,
    /// Initial virtual SOL reserves for price calculations
    pub initial_virtual_sol_reserves: u64,
    /// Initial actual token reserves available for trading
    pub initial_real_token_reserves: u64,
    /// Total supply of tokens
    pub token_total_supply: u64,
    /// Fee in basis points (1/100th of a percent)
    pub fee_basis_points: u64,
    /// Authority that can withdraw funds
    pub withdraw_authority: Pubkey,
    /// Flag to enable pool migration
    pub enable_migrate: bool,
    /// Fee for migrating pools
    pub pool_migration_fee: u64,
    /// Fee for creators in base points
    pub creator_fee_basis_points: u64,
    /// Array of public keys for fee recipients
    pub fee_recipients: [Pubkey; 7],
    /// Authority that sets the creator of the token
    pub set_creator_authority: Pubkey,
}

impl GlobalAccount {
    /// Creates a new global account instance
    ///
    /// # Arguments
    /// * `discriminator` - Unique identifier for the account
    /// * `initialized` - Whether the account is initialized
    /// * `authority` - Authority pubkey that can modify settings
    /// * `fee_recipient` - Account that receives fees
    /// * `initial_virtual_token_reserves` - Initial virtual token reserves
    /// * `initial_virtual_sol_reserves` - Initial virtual SOL reserves
    /// * `initial_real_token_reserves` - Initial actual token reserves
    /// * `token_total_supply` - Total supply of tokens
    /// * `fee_basis_points` - Fee in basis points
    /// * `withdraw_authority` - Authority that can withdraw funds
    /// * `enable_migrate` - Flag to enable pool migration
    /// * `pool_migration_fee` - Fee for migrating pools
    /// * `creator_fee_basis_points` - Fee for creators in base points
    /// * `fee_recipients` - Array of public keys for fee recipients
    /// * `set_creator_authority` - Authority that sets the creator of the token
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        discriminator: u64,
        initialized: bool,
        authority: Pubkey,
        fee_recipient: Pubkey,
        initial_virtual_token_reserves: u64,
        initial_virtual_sol_reserves: u64,
        initial_real_token_reserves: u64,
        token_total_supply: u64,
        fee_basis_points: u64,
        withdraw_authority: Pubkey,
        enable_migrate: bool,
        pool_migration_fee: u64,
        creator_fee_basis_points: u64,
        fee_recipients: [Pubkey; 7],
        set_creator_authority: Pubkey,
    ) -> Self {
        Self {
            discriminator,
            initialized,
            authority,
            fee_recipient,
            initial_virtual_token_reserves,
            initial_virtual_sol_reserves,
            initial_real_token_reserves,
            token_total_supply,
            fee_basis_points,
            withdraw_authority,
            enable_migrate,
            pool_migration_fee,
            creator_fee_basis_points,
            fee_recipients,
            set_creator_authority,
        }
    }

    /// Calculates the initial amount of tokens received for a given SOL amount
    ///
    /// # Arguments
    /// * `amount` - Amount of SOL to spend
    ///
    /// # Returns
    /// Amount of tokens that would be received
    pub fn get_initial_buy_price(&self, amount: u64) -> u64 {
        if amount == 0 {
            return 0;
        }

        let n: u128 = (self.initial_virtual_sol_reserves as u128)
            * (self.initial_virtual_token_reserves as u128);
        let i: u128 = (self.initial_virtual_sol_reserves as u128) + (amount as u128);
        let r: u128 = n / i + 1;
        let s: u128 = (self.initial_virtual_token_reserves as u128) - r;

        if s < (self.initial_real_token_reserves as u128) {
            s as u64
        } else {
            self.initial_real_token_reserves
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn get_global() -> GlobalAccount {
        GlobalAccount::new(
            1,
            true,
            Pubkey::new_unique(),
            Pubkey::new_unique(),
            1000,
            1000,
            500,
            1000,
            250,
            Pubkey::new_unique(),
            true,
            100,
            0,
            [Pubkey::new_unique(); 7],
            Pubkey::new_unique(),
        )
    }

    fn get_large_global() -> GlobalAccount {
        GlobalAccount::new(
            1,
            true,
            Pubkey::new_unique(),
            Pubkey::new_unique(),
            u64::MAX,
            u64::MAX,
            u64::MAX / 2,
            u64::MAX,
            250,
            Pubkey::new_unique(),
            true,
            u64::MAX,
            u64::MAX,
            [Pubkey::new_unique(); 7],
            Pubkey::new_unique(),
        )
    }

    #[test]
    fn test_global_account() {
        let global: GlobalAccount = get_global();

        // Test initial buy price calculation
        assert_eq!(global.get_initial_buy_price(0), 0);

        let price: u64 = global.get_initial_buy_price(100);
        assert!(price > 0);
        assert!(price <= global.initial_real_token_reserves);
    }

    #[test]
    fn test_global_account_max_reserves() {
        let mut global: GlobalAccount = get_global();
        global.initial_real_token_reserves = 100;

        // Test that returned amount is capped by real_token_reserves
        let price: u64 = global.get_initial_buy_price(1000);
        assert_eq!(price, global.initial_real_token_reserves);
    }

    #[test]
    fn test_global_account_overflow() {
        let global: GlobalAccount = get_large_global();

        // Test with maximum possible SOL amount
        let price: u64 = global.get_initial_buy_price(u64::MAX);
        assert!(price > 0);
        assert!(price <= global.initial_real_token_reserves);

        // Test with large but not maximum SOL amount
        let price: u64 = global.get_initial_buy_price(u64::MAX / 2);
        assert!(price > 0);
        assert!(price <= global.initial_real_token_reserves);
    }

    #[test]
    fn test_global_account_overflow_edge_cases() {
        let mut global: GlobalAccount = get_large_global();
        global.initial_virtual_sol_reserves = u64::MAX - 1000;
        global.initial_virtual_token_reserves = u64::MAX - 1000;
        global.initial_real_token_reserves = u64::MAX / 4;

        // Test with amounts near u64::MAX
        let price: u64 = global.get_initial_buy_price(u64::MAX - 1);
        assert!(price > 0);
        assert!(price <= global.initial_real_token_reserves);

        let price: u64 = global.get_initial_buy_price(u64::MAX - 1000);
        assert!(price > 0);
        assert!(price <= global.initial_real_token_reserves);
    }
}

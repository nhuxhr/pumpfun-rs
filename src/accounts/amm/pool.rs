use borsh::{BorshDeserialize, BorshSerialize};
use solana_sdk::pubkey::Pubkey;

/// Represents an AMM pool account for token swaps
#[derive(Debug, Clone, BorshSerialize, BorshDeserialize)]
pub struct PoolAccount {
    /// PDA bump seed for the pool account
    pub pool_bump: u8,

    /// Pool index number
    pub index: u16,

    /// Creator of the pool
    pub creator: Pubkey,

    /// Base token mint address
    pub base_mint: Pubkey,

    /// Quote token mint address
    pub quote_mint: Pubkey,

    /// LP token mint address
    pub lp_mint: Pubkey,

    /// Pool's base token account
    pub pool_base_token_account: Pubkey,

    /// Pool's quote token account
    pub pool_quote_token_account: Pubkey,

    /// True circulating supply without burns and lock-ups
    pub lp_supply: u64,
}

impl PoolAccount {
    /// Creates a new pool instance
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        pool_bump: u8,
        index: u16,
        creator: Pubkey,
        base_mint: Pubkey,
        quote_mint: Pubkey,
        lp_mint: Pubkey,
        pool_base_token_account: Pubkey,
        pool_quote_token_account: Pubkey,
        lp_supply: u64,
    ) -> Self {
        Self {
            pool_bump,
            index,
            creator,
            base_mint,
            quote_mint,
            lp_mint,
            pool_base_token_account,
            pool_quote_token_account,
            lp_supply,
        }
    }
}

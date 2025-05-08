use borsh::{BorshDeserialize, BorshSerialize};
use solana_sdk::pubkey::Pubkey;

/// Global configuration account for the AMM
#[derive(Debug, Clone, BorshSerialize, BorshDeserialize)]
pub struct GlobalConfigAccount {
    /// The admin pubkey
    pub admin: Pubkey,

    /// The lp fee in basis points (0.01%)
    pub lp_fee_basis_points: u64,

    /// The protocol fee in basis points (0.01%)
    pub protocol_fee_basis_points: u64,

    /// Flags to disable certain functionality
    /// bit 0 - Disable create pool
    /// bit 1 - Disable deposit
    /// bit 2 - Disable withdraw
    /// bit 3 - Disable buy
    /// bit 4 - Disable sell
    pub disable_flags: u8,

    /// Addresses of the protocol fee recipients
    pub protocol_fee_recipients: [Pubkey; 8],
}

impl GlobalConfigAccount {
    /// Creates a new global config instance
    pub fn new(
        admin: Pubkey,
        lp_fee_basis_points: u64,
        protocol_fee_basis_points: u64,
        disable_flags: u8,
        protocol_fee_recipients: [Pubkey; 8],
    ) -> Self {
        Self {
            admin,
            lp_fee_basis_points,
            protocol_fee_basis_points,
            disable_flags,
            protocol_fee_recipients,
        }
    }

    /// Constants for flag bits
    pub const CREATE_POOL_FLAG: u8 = 0;
    pub const DEPOSIT_FLAG: u8 = 1;
    pub const WITHDRAW_FLAG: u8 = 2;
    pub const BUY_FLAG: u8 = 3;
    pub const SELL_FLAG: u8 = 4;
}

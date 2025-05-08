use crate::{amm::PumpAmm, constants};
use borsh::{BorshDeserialize, BorshSerialize};
use solana_sdk::{
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
    signature::Keypair,
    signer::Signer,
};
use spl_associated_token_account::{
    get_associated_token_address, get_associated_token_address_with_program_id,
};

#[derive(BorshSerialize, BorshDeserialize, Clone)]
pub struct Withdraw {
    pub lp_token_amount_in: u64,
    pub min_base_amount_out: u64,
    pub min_quote_amount_out: u64,
}

impl Withdraw {
    pub const DISCRIMINATOR: [u8; 8] = [183, 18, 70, 156, 148, 109, 161, 34];

    pub fn data(&self) -> Vec<u8> {
        let mut data = Vec::with_capacity(256);
        data.extend_from_slice(&Self::DISCRIMINATOR);
        self.serialize(&mut data).unwrap();
        data
    }
}

pub fn withdraw(
    user: &Keypair,
    pool: &Pubkey,
    base_mint: &Pubkey,
    quote_mint: &Pubkey,
    base_token_program: &Pubkey,
    quote_token_program: &Pubkey,
    args: Withdraw,
) -> Instruction {
    let lp_mint = PumpAmm::get_lp_mint_pda(pool);

    Instruction::new_with_bytes(
        constants::accounts::amm::PUMPAMM,
        &args.data(),
        vec![
            AccountMeta::new(*pool, true),
            AccountMeta::new_readonly(PumpAmm::get_global_config_pda(), false),
            AccountMeta::new(user.pubkey(), true),
            AccountMeta::new_readonly(*base_mint, false),
            AccountMeta::new_readonly(*quote_mint, false),
            AccountMeta::new(lp_mint, true),
            AccountMeta::new(
                get_associated_token_address(&user.pubkey(), base_mint),
                true,
            ),
            AccountMeta::new(
                get_associated_token_address(&user.pubkey(), quote_mint),
                true,
            ),
            AccountMeta::new(
                get_associated_token_address_with_program_id(
                    &user.pubkey(),
                    &lp_mint,
                    &constants::accounts::TOKEN_2022_PROGRAM,
                ),
                true,
            ),
            AccountMeta::new(
                get_associated_token_address_with_program_id(pool, base_mint, base_token_program),
                true,
            ),
            AccountMeta::new(
                get_associated_token_address_with_program_id(pool, quote_mint, quote_token_program),
                true,
            ),
            AccountMeta::new_readonly(constants::accounts::TOKEN_PROGRAM, false),
            AccountMeta::new_readonly(constants::accounts::TOKEN_2022_PROGRAM, false),
            AccountMeta::new_readonly(PumpAmm::get_event_authority_pda(), false),
            AccountMeta::new_readonly(constants::accounts::amm::PUMPAMM, false),
        ],
    )
}

use crate::{amm::PumpAmm, constants};
use borsh::{BorshDeserialize, BorshSerialize};
use solana_sdk::{
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
    signature::Keypair,
    signer::Signer,
};
use spl_associated_token_account::get_associated_token_address_with_program_id;

#[derive(BorshSerialize, BorshDeserialize, Clone)]
pub struct CreatePool {
    pub index: u16,
    pub base_amount_in: u64,
    pub quote_amount_in: u64,
    pub coin_creator: Pubkey,
}

impl CreatePool {
    pub const DISCRIMINATOR: [u8; 8] = [233, 146, 209, 142, 207, 104, 64, 188];

    pub fn data(&self) -> Vec<u8> {
        let mut data = Vec::with_capacity(256);
        data.extend_from_slice(&Self::DISCRIMINATOR);
        self.serialize(&mut data).unwrap();
        data
    }
}

pub fn create_pool(
    creator: &Keypair,
    base_mint: &Pubkey,
    quote_mint: &Pubkey,
    base_token_program: &Pubkey,
    quote_token_program: &Pubkey,
    args: CreatePool,
) -> Instruction {
    let pool = PumpAmm::get_pool_pda(args.index, &creator.pubkey(), base_mint, quote_mint);
    let lp_mint = PumpAmm::get_lp_mint_pda(&pool);

    Instruction::new_with_bytes(
        constants::accounts::amm::PUMPAMM,
        &args.data(),
        vec![
            AccountMeta::new(pool, false),
            AccountMeta::new_readonly(PumpAmm::get_global_config_pda(), false),
            AccountMeta::new(creator.pubkey(), true),
            AccountMeta::new_readonly(*base_mint, false),
            AccountMeta::new_readonly(*quote_mint, false),
            AccountMeta::new(lp_mint, false),
            AccountMeta::new(
                get_associated_token_address_with_program_id(
                    &creator.pubkey(),
                    base_mint,
                    base_token_program,
                ),
                false,
            ),
            AccountMeta::new(
                get_associated_token_address_with_program_id(
                    &creator.pubkey(),
                    quote_mint,
                    quote_token_program,
                ),
                false,
            ),
            AccountMeta::new(
                get_associated_token_address_with_program_id(
                    &creator.pubkey(),
                    &lp_mint,
                    &constants::accounts::TOKEN_2022_PROGRAM,
                ),
                false,
            ),
            AccountMeta::new(
                get_associated_token_address_with_program_id(&pool, base_mint, base_token_program),
                false,
            ),
            AccountMeta::new(
                get_associated_token_address_with_program_id(
                    &pool,
                    quote_mint,
                    quote_token_program,
                ),
                false,
            ),
            AccountMeta::new_readonly(constants::accounts::SYSTEM_PROGRAM, false),
            AccountMeta::new_readonly(constants::accounts::TOKEN_2022_PROGRAM, false),
            AccountMeta::new_readonly(*base_token_program, false),
            AccountMeta::new_readonly(*quote_token_program, false),
            AccountMeta::new_readonly(constants::accounts::ASSOCIATED_TOKEN_PROGRAM, false),
            AccountMeta::new_readonly(PumpAmm::get_event_authority_pda(), false),
            AccountMeta::new_readonly(constants::accounts::amm::PUMPAMM, false),
        ],
    )
}

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
pub struct Sell {
    pub base_amount_in: u64,
    pub min_quote_amount_out: u64,
}

impl Sell {
    pub const DISCRIMINATOR: [u8; 8] = [51, 230, 133, 164, 1, 127, 131, 173];

    pub fn data(&self) -> Vec<u8> {
        let mut data = Vec::with_capacity(256);
        data.extend_from_slice(&Self::DISCRIMINATOR);
        self.serialize(&mut data).unwrap();
        data
    }
}

#[allow(clippy::too_many_arguments)]
pub fn sell(
    user: &Keypair,
    pool: &Pubkey,
    base_mint: &Pubkey,
    quote_mint: &Pubkey,
    base_token_program: &Pubkey,
    quote_token_program: &Pubkey,
    protocol_fee_recipient: &Pubkey,
    coin_creator: &Pubkey,
    args: Sell,
) -> Instruction {
    let coin_creator_vault_authority = PumpAmm::get_coin_creator_vault_authority_pda(coin_creator);

    Instruction::new_with_bytes(
        constants::accounts::amm::PUMPAMM,
        &args.data(),
        vec![
            AccountMeta::new(*pool, false),
            AccountMeta::new(user.pubkey(), true),
            AccountMeta::new_readonly(PumpAmm::get_global_config_pda(), false),
            AccountMeta::new_readonly(*base_mint, false),
            AccountMeta::new_readonly(*quote_mint, false),
            AccountMeta::new(
                get_associated_token_address_with_program_id(
                    &user.pubkey(),
                    base_mint,
                    base_token_program,
                ),
                false,
            ),
            AccountMeta::new(
                get_associated_token_address_with_program_id(
                    &user.pubkey(),
                    quote_mint,
                    quote_token_program,
                ),
                false,
            ),
            AccountMeta::new(
                get_associated_token_address_with_program_id(pool, base_mint, base_token_program),
                false,
            ),
            AccountMeta::new(
                get_associated_token_address_with_program_id(pool, quote_mint, quote_token_program),
                false,
            ),
            AccountMeta::new_readonly(*protocol_fee_recipient, false),
            AccountMeta::new(
                get_associated_token_address_with_program_id(
                    protocol_fee_recipient,
                    quote_mint,
                    quote_token_program,
                ),
                false,
            ),
            AccountMeta::new_readonly(*base_token_program, false),
            AccountMeta::new_readonly(*quote_token_program, false),
            AccountMeta::new_readonly(constants::accounts::SYSTEM_PROGRAM, false),
            AccountMeta::new_readonly(constants::accounts::ASSOCIATED_TOKEN_PROGRAM, false),
            AccountMeta::new_readonly(PumpAmm::get_event_authority_pda(), false),
            AccountMeta::new_readonly(constants::accounts::amm::PUMPAMM, false),
            AccountMeta::new(
                get_associated_token_address_with_program_id(
                    &coin_creator_vault_authority,
                    quote_mint,
                    quote_token_program,
                ),
                false,
            ),
            AccountMeta::new_readonly(coin_creator_vault_authority, false),
        ],
    )
}

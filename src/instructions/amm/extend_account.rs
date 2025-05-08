use crate::{amm::PumpAmm, constants};
use borsh::{BorshDeserialize, BorshSerialize};
use solana_sdk::{
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
    signature::Keypair,
    signer::Signer,
};

#[derive(BorshSerialize, BorshDeserialize, Clone)]
pub struct ExtendAccount {}

impl ExtendAccount {
    pub const DISCRIMINATOR: [u8; 8] = [234, 102, 194, 203, 150, 72, 62, 229];

    pub fn data(&self) -> Vec<u8> {
        let mut data = Vec::with_capacity(256);
        data.extend_from_slice(&Self::DISCRIMINATOR);
        self.serialize(&mut data).unwrap();
        data
    }
}

pub fn extend_account(user: &Keypair, account: &Pubkey, args: ExtendAccount) -> Instruction {
    Instruction::new_with_bytes(
        constants::accounts::amm::PUMPAMM,
        &args.data(),
        vec![
            AccountMeta::new(*account, false),
            AccountMeta::new(user.pubkey(), true),
            AccountMeta::new_readonly(constants::accounts::SYSTEM_PROGRAM, false),
            AccountMeta::new_readonly(PumpAmm::get_event_authority_pda(), false),
            AccountMeta::new_readonly(constants::accounts::amm::PUMPAMM, false),
        ],
    )
}

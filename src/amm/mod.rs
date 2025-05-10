use std::sync::Arc;

use futures::future::try_join_all;
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::{
    account::Account, instruction::Instruction, pubkey::Pubkey, signature::Keypair, signer::Signer,
    system_instruction,
};
use spl_associated_token_account::{
    get_associated_token_address_with_program_id,
    instruction::create_associated_token_account_idempotent,
};
use spl_token::{instruction::sync_native, native_mint};

use crate::{
    accounts, common::types::Cluster, constants, error, instructions, utils::get_mint_token_program,
};

pub struct PumpAmm {
    /// Keypair used to sign transactions
    pub payer: Arc<Keypair>,
    /// RPC client for Solana network requests
    pub rpc: Arc<RpcClient>,
    /// Cluster configuration
    pub cluster: Cluster,
}

impl PumpAmm {
    pub fn new(payer: Arc<Keypair>, cluster: Cluster) -> Self {
        // Create Solana RPC Client with HTTP endpoint
        let rpc = Arc::new(RpcClient::new_with_commitment(
            cluster.rpc.http.clone(),
            cluster.commitment,
        ));

        // Return configured PumpFun client
        Self {
            payer,
            rpc,
            cluster,
        }
    }

    pub fn create_pool() {}

    pub fn deposit() {}

    pub fn withdraw() {}

    pub fn buy() {}

    pub fn sell() {}

    pub fn extend_account() {}

    pub async fn get_create_pool_instructions(
        &self,
        index: u16,
        base_mint: Pubkey,
        quote_mint: Pubkey,
        base_amount_in: u64,
        quote_amount_in: u64,
    ) -> Result<Vec<Instruction>, error::ClientError> {
        let pool_pda = Self::get_pool_pda(index, &self.payer.pubkey(), &base_mint, &quote_mint);
        let mint_token_programs = try_join_all(vec![
            get_mint_token_program(self.rpc.clone(), &base_mint),
            get_mint_token_program(self.rpc.clone(), &quote_mint),
        ])
        .await?;
        let base_token_program = mint_token_programs[0];
        let quote_token_program = mint_token_programs[1];
        let user_quote_token_account = get_associated_token_address_with_program_id(
            &self.payer.pubkey(),
            &quote_mint,
            &quote_token_program,
        );
        let pool_base_token_account = get_associated_token_address_with_program_id(
            &pool_pda,
            &base_mint,
            &base_token_program,
        );
        let pool_quote_token_account = get_associated_token_address_with_program_id(
            &pool_pda,
            &quote_mint,
            &quote_token_program,
        );

        let mut instructions = self
            .get_with_wsol_instructions(quote_mint, user_quote_token_account, quote_amount_in)
            .await?;

        if self
            .rpc
            .get_account(&pool_base_token_account)
            .await
            .is_err()
        {
            instructions.push(create_associated_token_account_idempotent(
                &self.payer.pubkey(),
                &pool_base_token_account,
                &base_mint,
                &base_token_program,
            ));
        }

        if self
            .rpc
            .get_account(&pool_quote_token_account)
            .await
            .is_err()
        {
            instructions.push(create_associated_token_account_idempotent(
                &self.payer.pubkey(),
                &pool_quote_token_account,
                &base_mint,
                &base_token_program,
            ));
        }

        instructions.push(instructions::amm::create_pool(
            &self.payer.clone(),
            &base_mint,
            &quote_mint,
            &base_token_program,
            &quote_token_program,
            instructions::amm::CreatePool {
                index,
                base_amount_in,
                quote_amount_in,
            },
        ));

        Ok(instructions)
    }

    pub async fn get_deposit_instructions(
        &self,
        pool: Pubkey,
        lp_token: u64,
        max_base: u64,
        max_quote: u64,
    ) -> Result<Vec<Instruction>, error::ClientError> {
        let pool_account = self.get_pool_account(&pool).await?;
        let mint_token_programs = try_join_all(vec![
            get_mint_token_program(self.rpc.clone(), &pool_account.1.base_mint),
            get_mint_token_program(self.rpc.clone(), &pool_account.1.quote_mint),
        ])
        .await?;
        let base_token_program = mint_token_programs[0];
        let quote_token_program = mint_token_programs[1];
        let user_quote_token_account = get_associated_token_address_with_program_id(
            &self.payer.pubkey(),
            &pool_account.1.quote_mint,
            &quote_token_program,
        );
        let user_pool_token_account = get_associated_token_address_with_program_id(
            &self.payer.pubkey(),
            &pool_account.1.lp_mint,
            &constants::accounts::TOKEN_2022_PROGRAM,
        );

        let mut instructions = self
            .get_with_wsol_instructions(
                pool_account.1.quote_mint,
                user_quote_token_account,
                max_quote,
            )
            .await?;

        if pool_account
            .0
            .data
            .len()
            .lt(&(constants::POOL_ACCOUNT_SIZE as usize))
        {
            instructions.push(self.get_extend_account_instruction(pool));
        }

        if self
            .rpc
            .get_account(&user_pool_token_account)
            .await
            .is_err()
        {
            instructions.push(create_associated_token_account_idempotent(
                &self.payer.pubkey(),
                &user_pool_token_account,
                &pool_account.1.lp_mint,
                &constants::accounts::TOKEN_2022_PROGRAM,
            ));
        }

        instructions.push(instructions::amm::deposit(
            &self.payer.clone(),
            &pool,
            &pool_account.1.base_mint,
            &pool_account.1.quote_mint,
            &base_token_program,
            &quote_token_program,
            instructions::amm::Deposit {
                lp_token_amount_out: lp_token,
                max_base_amount_in: max_base,
                max_quote_amount_in: max_quote,
            },
        ));

        Ok(instructions)
    }

    pub async fn get_withdraw_instructions(
        &self,
        pool: Pubkey,
        lp_token_amount_in: u64,
        min_base_amount_out: u64,
        min_quote_amount_out: u64,
    ) -> Result<Vec<Instruction>, error::ClientError> {
        let pool_account = self.get_pool_account(&pool).await?;
        let mint_token_programs = try_join_all(vec![
            get_mint_token_program(self.rpc.clone(), &pool_account.1.base_mint),
            get_mint_token_program(self.rpc.clone(), &pool_account.1.quote_mint),
        ])
        .await?;
        let base_token_program = mint_token_programs[0];
        let quote_token_program = mint_token_programs[1];
        let user_base_token_account = get_associated_token_address_with_program_id(
            &self.payer.pubkey(),
            &pool_account.1.base_mint,
            &base_token_program,
        );
        let user_quote_token_account = get_associated_token_address_with_program_id(
            &self.payer.pubkey(),
            &pool_account.1.quote_mint,
            &quote_token_program,
        );

        let mut instructions = vec![];

        if pool_account
            .0
            .data
            .len()
            .lt(&(constants::POOL_ACCOUNT_SIZE as usize))
        {
            instructions.push(self.get_extend_account_instruction(pool));
        }

        if self
            .rpc
            .get_account(&user_base_token_account)
            .await
            .is_err()
        {
            instructions.push(create_associated_token_account_idempotent(
                &self.payer.pubkey(),
                &user_base_token_account,
                &pool_account.1.base_mint,
                &base_token_program,
            ));
        }

        if self
            .rpc
            .get_account(&user_quote_token_account)
            .await
            .is_err()
        {
            instructions.push(create_associated_token_account_idempotent(
                &self.payer.pubkey(),
                &user_quote_token_account,
                &pool_account.1.quote_mint,
                &quote_token_program,
            ));
        }

        instructions.push(instructions::amm::withdraw(
            &self.payer.clone(),
            &pool,
            &pool_account.1.base_mint,
            &pool_account.1.quote_mint,
            &base_token_program,
            &quote_token_program,
            instructions::amm::Withdraw {
                lp_token_amount_in,
                min_base_amount_out,
                min_quote_amount_out,
            },
        ));

        Ok(instructions)
    }

    pub async fn get_buy_instructions(
        &self,
        pool: Pubkey,
        base_out: u64,
        max_quote_in: u64,
        protocol_fee_recipient: Option<Pubkey>,
    ) -> Result<Vec<Instruction>, error::ClientError> {
        let protocol_fee_recipient = match protocol_fee_recipient {
            Some(protocol_fee_recipient) => protocol_fee_recipient,
            None => {
                self.get_global_config_account()
                    .await?
                    .1
                    .protocol_fee_recipients[0]
            }
        };
        let pool_account = self.get_pool_account(&pool).await?;
        let mint_token_programs = try_join_all(vec![
            get_mint_token_program(self.rpc.clone(), &pool_account.1.base_mint),
            get_mint_token_program(self.rpc.clone(), &pool_account.1.quote_mint),
        ])
        .await?;
        let base_token_program = mint_token_programs[0];
        let quote_token_program = mint_token_programs[1];
        let user_quote_token_account = get_associated_token_address_with_program_id(
            &self.payer.pubkey(),
            &pool_account.1.quote_mint,
            &quote_token_program,
        );

        let mut instructions = self
            .get_with_wsol_instructions(
                pool_account.1.quote_mint,
                user_quote_token_account,
                max_quote_in,
            )
            .await?;

        if pool_account
            .0
            .data
            .len()
            .lt(&(constants::POOL_ACCOUNT_SIZE as usize))
        {
            instructions.push(self.get_extend_account_instruction(pool));
        }

        instructions.push(instructions::amm::buy(
            &self.payer.clone(),
            &pool,
            &pool_account.1.base_mint,
            &pool_account.1.quote_mint,
            &base_token_program,
            &quote_token_program,
            &protocol_fee_recipient,
            instructions::amm::Buy {
                base_amount_out: base_out,
                max_quote_amount_in: max_quote_in,
            },
        ));

        Ok(instructions)
    }

    pub fn get_sell_instructions() {}

    pub fn get_extend_account_instruction(&self, account: Pubkey) -> Instruction {
        instructions::amm::extend_account(
            &self.payer,
            &account,
            instructions::amm::ExtendAccount {},
        )
    }

    pub async fn get_with_wsol_instructions(
        &self,
        mint: Pubkey,
        ata: Pubkey,
        amount: u64,
    ) -> Result<Vec<Instruction>, error::ClientError> {
        let mut instructions = vec![];

        if mint.eq(&native_mint::ID) {
            #[cfg(feature = "create-ata")]
            if self.rpc.get_account(&ata).await.is_err() {
                instructions.push(create_associated_token_account_idempotent(
                    &self.payer.pubkey(),
                    &ata,
                    &native_mint::ID,
                    &constants::accounts::TOKEN_PROGRAM,
                ));
            }
            if amount.gt(&0) {
                instructions.push(system_instruction::transfer(
                    &self.payer.pubkey(),
                    &ata,
                    amount,
                ));
                instructions.push(
                    sync_native(&constants::accounts::TOKEN_PROGRAM, &ata).map_err(|err| {
                        error::ClientError::OtherError(format!(
                            "Failed to sync native mint: {}",
                            err
                        ))
                    })?,
                );
            }
        }

        Ok(instructions)
    }

    pub fn get_global_config_pda() -> Pubkey {
        let seeds: &[&[u8]; 1] = &[constants::seeds::amm::GLOBAL_CONFIG_SEED];
        let program_id: &Pubkey = &constants::accounts::amm::PUMPAMM;
        Pubkey::find_program_address(seeds, program_id).0
    }

    pub fn get_pool_pda(
        index: u16,
        owner: &Pubkey,
        base_mint: &Pubkey,
        quote_mint: &Pubkey,
    ) -> Pubkey {
        let seeds: &[&[u8]] = &[
            constants::seeds::amm::POOL_SEED,
            &index.to_le_bytes(),
            owner.as_ref(),
            base_mint.as_ref(),
            quote_mint.as_ref(),
        ];
        Pubkey::find_program_address(seeds, &constants::accounts::amm::PUMPAMM).0
    }

    pub fn get_lp_mint_pda(pool: &Pubkey) -> Pubkey {
        let seeds: &[&[u8]] = &[constants::seeds::amm::POOL_LP_MINT_SEED, pool.as_ref()];
        Pubkey::find_program_address(seeds, &constants::accounts::amm::PUMPAMM).0
    }

    pub fn get_pool_authority_pda(mint: &Pubkey) -> Pubkey {
        let seeds: &[&[u8]] = &[constants::seeds::amm::POOL_AUTHORITY_SEED, mint.as_ref()];
        Pubkey::find_program_address(seeds, &constants::accounts::PUMPFUN).0
    }

    pub fn get_event_authority_pda() -> Pubkey {
        let seeds: &[&[u8]] = &[constants::seeds::amm::EVENT_AUTHORITY_SEED];
        Pubkey::find_program_address(seeds, &constants::accounts::amm::PUMPAMM).0
    }

    pub async fn get_global_config_account(
        &self,
    ) -> Result<(Account, accounts::amm::GlobalConfigAccount), error::ClientError> {
        let global_config: Pubkey = Self::get_global_config_pda();

        let account = self
            .rpc
            .get_account(&global_config)
            .await
            .map_err(error::ClientError::SolanaClientError)?;

        Ok((
            account.clone(),
            solana_sdk::borsh1::try_from_slice_unchecked::<accounts::amm::GlobalConfigAccount>(
                &account.data[8..],
            )
            .map_err(error::ClientError::BorshError)?,
        ))
    }

    pub async fn get_pool_account(
        &self,
        pool: &Pubkey,
    ) -> Result<(Account, accounts::amm::PoolAccount), error::ClientError> {
        let account = self
            .rpc
            .get_account(pool)
            .await
            .map_err(error::ClientError::SolanaClientError)?;

        Ok((
            account.clone(),
            solana_sdk::borsh1::try_from_slice_unchecked::<accounts::amm::PoolAccount>(
                &account.data[8..],
            )
            .map_err(error::ClientError::BorshError)?,
        ))
    }
}

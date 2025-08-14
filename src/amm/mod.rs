use std::sync::Arc;

use futures::future::try_join_all;
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::{
    account::Account,
    instruction::Instruction,
    pubkey::Pubkey,
    signature::{Keypair, Signature},
    signer::Signer,
    system_instruction, system_program,
};
use spl_associated_token_account::{
    get_associated_token_address_with_program_id,
    instruction::create_associated_token_account_idempotent,
};
use spl_token::{instruction::sync_native, native_mint};

use crate::{
    accounts,
    common::types::{Cluster, PriorityFee, SwapDirection, SwapInput},
    constants, error, instructions,
    utils::{self, get_mint_token_program, transaction::get_transaction},
    PumpFun,
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

    pub async fn create_pool(
        &self,
        index: u16,
        base_mint: Pubkey,
        quote_mint: Pubkey,
        base_amount_in: u64,
        quote_amount_in: u64,
        priority_fee: Option<PriorityFee>,
    ) -> Result<Signature, error::ClientError> {
        let priority_fee = priority_fee.unwrap_or(self.cluster.priority_fee);
        let mut instructions = PumpFun::get_priority_fee_instructions(&priority_fee);

        let create_pool_ixs = self
            .get_create_pool_instructions(
                index,
                base_mint,
                quote_mint,
                base_amount_in,
                quote_amount_in,
            )
            .await?;
        instructions.extend(create_pool_ixs);

        let transaction = get_transaction(
            self.rpc.clone(),
            self.payer.clone(),
            &instructions,
            None,
            #[cfg(feature = "versioned-tx")]
            None,
        )
        .await?;

        let signature = self
            .rpc
            .send_and_confirm_transaction(&transaction)
            .await
            .map_err(error::ClientError::SolanaClientError)?;

        Ok(signature)
    }

    pub async fn deposit(
        &self,
        pool: Pubkey,
        lp_token: u64,
        slippage: u8,
        priority_fee: Option<PriorityFee>,
    ) -> Result<Signature, error::ClientError> {
        let (pool_account, pool_base_balance, pool_quote_balance) =
            self.get_pool_balances(&pool).await?;
        let (max_base, max_quote) = utils::amm::deposit::deposit_lp_token(
            lp_token,
            slippage,
            pool_base_balance,
            pool_quote_balance,
            pool_account.lp_supply,
        )?;

        let priority_fee = priority_fee.unwrap_or(self.cluster.priority_fee);
        let mut instructions = PumpFun::get_priority_fee_instructions(&priority_fee);

        let deposit_ixs = self
            .get_deposit_instructions(pool, lp_token, max_base, max_quote)
            .await?;
        instructions.extend(deposit_ixs);

        let transaction = get_transaction(
            self.rpc.clone(),
            self.payer.clone(),
            &instructions,
            None,
            #[cfg(feature = "versioned-tx")]
            None,
        )
        .await?;

        let signature = self
            .rpc
            .send_and_confirm_transaction(&transaction)
            .await
            .map_err(error::ClientError::SolanaClientError)?;

        Ok(signature)
    }

    pub async fn withdraw(
        &self,
        pool: Pubkey,
        lp_token: u64,
        slippage: u8,
        priority_fee: Option<PriorityFee>,
    ) -> Result<Signature, error::ClientError> {
        let (pool_account, pool_base_balance, pool_quote_balance) =
            self.get_pool_balances(&pool).await?;
        let (_, _, min_base, min_quote) = utils::amm::withdraw::withdraw_lp_token(
            lp_token,
            slippage,
            pool_base_balance,
            pool_quote_balance,
            pool_account.lp_supply,
        )?;

        let priority_fee = priority_fee.unwrap_or(self.cluster.priority_fee);
        let mut instructions = PumpFun::get_priority_fee_instructions(&priority_fee);

        let withdraw_ixs = self
            .get_withdraw_instructions(pool, lp_token, min_base, min_quote)
            .await?;
        instructions.extend(withdraw_ixs);

        let transaction = get_transaction(
            self.rpc.clone(),
            self.payer.clone(),
            &instructions,
            None,
            #[cfg(feature = "versioned-tx")]
            None,
        )
        .await?;

        let signature = self
            .rpc
            .send_and_confirm_transaction(&transaction)
            .await
            .map_err(error::ClientError::SolanaClientError)?;

        Ok(signature)
    }

    pub async fn swap(
        &self,
        pool: Pubkey,
        amount: u64,
        slippage: u8,
        swap_input: SwapInput,
        swap_direction: SwapDirection,
        priority_fee: Option<PriorityFee>,
    ) -> Result<Signature, error::ClientError> {
        let global = self.get_global_config_account().await?.1;
        let (pool_account, pool_base_balance, pool_quote_balance) =
            self.get_pool_balances(&pool).await?;

        let priority_fee = priority_fee.unwrap_or(self.cluster.priority_fee);
        let mut instructions = PumpFun::get_priority_fee_instructions(&priority_fee);

        let swap_ixs = match swap_input {
            SwapInput::Base => match swap_direction {
                SwapDirection::QuoteToBase => {
                    let (_, _, max_quote) = utils::amm::buy::buy_base_input(
                        amount,
                        slippage,
                        pool_base_balance,
                        pool_quote_balance,
                        global.lp_fee_basis_points,
                        global.protocol_fee_basis_points,
                        global.coin_creator_fee_basis_points,
                        &pool_account.coin_creator,
                    )?;

                    self.get_buy_instructions(pool, amount, max_quote, None)
                        .await?
                }
                SwapDirection::BaseToQuote => {
                    let (_, _, min_quote) = utils::amm::sell::sell_base_input(
                        amount,
                        slippage,
                        pool_base_balance,
                        pool_quote_balance,
                        global.lp_fee_basis_points,
                        global.protocol_fee_basis_points,
                        global.coin_creator_fee_basis_points,
                        &pool_account.coin_creator,
                    )?;

                    self.get_sell_instructions(pool, amount, min_quote, None)
                        .await?
                }
            },
            SwapInput::Quote => match swap_direction {
                SwapDirection::QuoteToBase => {
                    let (_, base, max_quote) = utils::amm::buy::buy_quote_input(
                        amount,
                        slippage,
                        pool_base_balance,
                        pool_quote_balance,
                        global.lp_fee_basis_points,
                        global.protocol_fee_basis_points,
                        global.coin_creator_fee_basis_points,
                        &pool_account.coin_creator,
                    )?;

                    self.get_buy_instructions(pool, base, max_quote, None)
                        .await?
                }
                SwapDirection::BaseToQuote => {
                    let (_, base, min_quote) = utils::amm::sell::sell_quote_input(
                        amount,
                        slippage,
                        pool_base_balance,
                        pool_quote_balance,
                        global.lp_fee_basis_points,
                        global.protocol_fee_basis_points,
                        global.coin_creator_fee_basis_points,
                        &pool_account.coin_creator,
                    )?;

                    self.get_sell_instructions(pool, base, min_quote, None)
                        .await?
                }
            },
        };
        instructions.extend(swap_ixs);

        let transaction = get_transaction(
            self.rpc.clone(),
            self.payer.clone(),
            &instructions,
            None,
            #[cfg(feature = "versioned-tx")]
            None,
        )
        .await?;

        let signature = self
            .rpc
            .send_and_confirm_transaction(&transaction)
            .await
            .map_err(error::ClientError::SolanaClientError)?;

        Ok(signature)
    }

    pub async fn extend_account(
        &self,
        pool: Pubkey,
        priority_fee: Option<PriorityFee>,
    ) -> Result<Signature, error::ClientError> {
        let priority_fee = priority_fee.unwrap_or(self.cluster.priority_fee);
        let mut instructions = PumpFun::get_priority_fee_instructions(&priority_fee);
        instructions.push(self.get_extend_account_instruction(pool));

        let transaction = get_transaction(
            self.rpc.clone(),
            self.payer.clone(),
            &instructions,
            None,
            #[cfg(feature = "versioned-tx")]
            None,
        )
        .await?;

        let signature = self
            .rpc
            .send_and_confirm_transaction(&transaction)
            .await
            .map_err(error::ClientError::SolanaClientError)?;

        Ok(signature)
    }

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
                &pool_pda,
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
                &pool_pda,
                &quote_mint,
                &quote_token_program,
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
                coin_creator: system_program::ID,
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
                &self.payer.pubkey(),
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
                &self.payer.pubkey(),
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
                &self.payer.pubkey(),
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
            &pool_account.1.coin_creator,
            instructions::amm::Buy {
                base_amount_out: base_out,
                max_quote_amount_in: max_quote_in,
            },
        ));

        Ok(instructions)
    }

    pub async fn get_sell_instructions(
        &self,
        pool: Pubkey,
        base_amount_in: u64,
        min_quote_amount_out: u64,
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
        let user_base_token_account = get_associated_token_address_with_program_id(
            &self.payer.pubkey(),
            &pool_account.1.base_mint,
            &base_token_program,
        );

        let mut instructions = self
            .get_with_wsol_instructions(
                pool_account.1.base_mint,
                user_base_token_account,
                base_amount_in,
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

        instructions.push(instructions::amm::sell(
            &self.payer.clone(),
            &pool,
            &pool_account.1.base_mint,
            &pool_account.1.quote_mint,
            &base_token_program,
            &quote_token_program,
            &protocol_fee_recipient,
            &pool_account.1.coin_creator,
            instructions::amm::Sell {
                base_amount_in,
                min_quote_amount_out,
            },
        ));

        Ok(instructions)
    }

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
                    &self.payer.pubkey(),
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

    pub fn get_coin_creator_vault_authority_pda(coin_creator: &Pubkey) -> Pubkey {
        let seeds: &[&[u8]] = &[constants::seeds::amm::CREATOR_VAULT, coin_creator.as_ref()];
        Pubkey::find_program_address(seeds, &constants::accounts::amm::PUMPAMM).0
    }

    pub fn get_user_volume_accumulator_pda(user: &Pubkey) -> Pubkey {
        let (user_volume_accumulator, _bump) = Pubkey::find_program_address(
            &[b"user_volume_accumulator", user.as_ref()],
            &constants::accounts::amm::PUMPAMM,
        );
        user_volume_accumulator
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

    pub async fn get_pool_balances(
        &self,
        pool: &Pubkey,
    ) -> Result<(accounts::amm::PoolAccount, u64, u64), error::ClientError> {
        let pool = self.get_pool_account(pool).await?.1;

        let rpc = self.rpc.clone();
        let mint_token_balances = try_join_all({
            vec![
                rpc.get_token_account_balance(&pool.pool_base_token_account),
                rpc.get_token_account_balance(&pool.pool_quote_token_account),
            ]
        })
        .await?;
        let base_token_balance = mint_token_balances[0]
            .clone()
            .amount
            .parse::<u64>()
            .unwrap();
        let quote_token_balance = mint_token_balances[1]
            .clone()
            .amount
            .parse::<u64>()
            .unwrap();

        Ok((pool, base_token_balance, quote_token_balance))
    }
}

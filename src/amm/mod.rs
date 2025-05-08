use std::sync::Arc;

use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::{pubkey::Pubkey, signature::Keypair};

use crate::{accounts, common::types::Cluster, constants, error};

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

    pub fn get_create_pool_instructions() {}

    pub fn get_deposit_instructions() {}

    pub fn get_withdraw_instructions() {}

    pub fn get_buy_instructions() {}

    pub fn get_sell_instructions() {}

    pub fn get_extend_account_instruction() {}

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
    ) -> Result<accounts::amm::GlobalConfigAccount, error::ClientError> {
        let global_config: Pubkey = Self::get_global_config_pda();

        let account = self
            .rpc
            .get_account(&global_config)
            .await
            .map_err(error::ClientError::SolanaClientError)?;

        solana_sdk::borsh1::try_from_slice_unchecked::<accounts::amm::GlobalConfigAccount>(
            &account.data[8..],
        )
        .map_err(error::ClientError::BorshError)
    }

    pub async fn get_pool_account(
        &self,
        pool: &Pubkey,
    ) -> Result<accounts::amm::PoolAccount, error::ClientError> {
        let account = self
            .rpc
            .get_account(pool)
            .await
            .map_err(error::ClientError::SolanaClientError)?;

        solana_sdk::borsh1::try_from_slice_unchecked::<accounts::amm::PoolAccount>(
            &account.data[8..],
        )
        .map_err(error::ClientError::BorshError)
    }
}

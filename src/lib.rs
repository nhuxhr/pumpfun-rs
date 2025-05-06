#![doc = include_str!("../RUSTDOC.md")]

pub mod accounts;
#[cfg(feature = "amm")]
pub mod amm;
pub mod common;
pub mod constants;
pub mod error;
pub mod instructions;
pub mod utils;

use common::types::{Cluster, PriorityFee};
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::{
    compute_budget::ComputeBudgetInstruction,
    instruction::Instruction,
    pubkey::Pubkey,
    signature::{Keypair, Signature},
    signer::Signer,
};
use spl_associated_token_account::get_associated_token_address;
#[cfg(feature = "create-ata")]
use spl_associated_token_account::instruction::create_associated_token_account;
#[cfg(feature = "close-ata")]
use spl_token::instruction::close_account;
use std::sync::Arc;
use utils::transaction::get_transaction;

/// Main client for interacting with the Pump.fun program
///
/// This struct provides the primary interface for interacting with the Pump.fun
/// token platform on Solana. It handles connection to the Solana network and provides
/// methods for token creation, buying, and selling using bonding curves.
///
/// # Examples
///
/// ```no_run
/// use pumpfun::{PumpFun, common::types::{Cluster, PriorityFee}};
/// use solana_sdk::{commitment_config::CommitmentConfig, signature::Keypair};
/// use std::sync::Arc;
///
/// // Create a new client connected to devnet
/// let payer = Arc::new(Keypair::new());
/// let commitment = CommitmentConfig::confirmed();
/// let priority_fee = PriorityFee::default();
/// let cluster = Cluster::devnet(commitment, priority_fee);
/// let client = PumpFun::new(payer, cluster);
/// ```
pub struct PumpFun {
    /// Keypair used to sign transactions
    pub payer: Arc<Keypair>,
    /// RPC client for Solana network requests
    pub rpc: Arc<RpcClient>,
    /// Cluster configuration
    pub cluster: Cluster,
    #[cfg(feature = "amm")]
    pub amm: amm::PumpAmm,
}

impl PumpFun {
    /// Creates a new PumpFun client instance
    ///
    /// Initializes a new client for interacting with the Pump.fun program on Solana.
    /// This client manages connection to the Solana network and provides methods for
    /// creating, buying, and selling tokens.
    ///
    /// # Arguments
    ///
    /// * `payer` - Keypair used to sign and pay for transactions
    /// * `cluster` - Solana cluster configuration including RPC endpoints and transaction parameters
    ///
    /// # Returns
    ///
    /// Returns a new PumpFun client instance configured with the provided parameters
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use pumpfun::{PumpFun, common::types::{Cluster, PriorityFee}};
    /// use solana_sdk::{commitment_config::CommitmentConfig, signature::Keypair};
    /// use std::sync::Arc;
    ///
    /// let payer = Arc::new(Keypair::new());
    /// let commitment = CommitmentConfig::confirmed();
    /// let priority_fee = PriorityFee::default();
    /// let cluster = Cluster::devnet(commitment, priority_fee);
    /// let client = PumpFun::new(payer, cluster);
    /// ```
    pub fn new(payer: Arc<Keypair>, cluster: Cluster) -> Self {
        // Create Solana RPC Client with HTTP endpoint
        let rpc = Arc::new(RpcClient::new_with_commitment(
            cluster.rpc.http.clone(),
            cluster.commitment,
        ));

        // Return configured PumpFun client
        Self {
            payer: payer.clone(),
            rpc: rpc.clone(),
            cluster: cluster.clone(),
            #[cfg(feature = "amm")]
            amm: amm::PumpAmm {
                payer,
                rpc,
                cluster,
            },
        }
    }

    /// Creates a new token with metadata by uploading metadata to IPFS and initializing on-chain accounts
    ///
    /// This method handles the complete process of creating a new token on Pump.fun:
    /// 1. Uploads token metadata and image to IPFS
    /// 2. Creates a new SPL token with the provided mint keypair
    /// 3. Initializes the bonding curve that determines token pricing
    /// 4. Sets up metadata using the Metaplex standard
    ///
    /// # Arguments
    ///
    /// * `mint` - Keypair for the new token mint account that will be created
    /// * `metadata` - Token metadata including name, symbol, description and image file
    /// * `priority_fee` - Optional priority fee configuration for compute units. If None, uses the
    ///   default from the cluster configuration
    ///
    /// # Returns
    ///
    /// Returns the transaction signature if successful, or a ClientError if the operation fails
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Metadata upload to IPFS fails
    /// - Transaction creation fails
    /// - Transaction execution on Solana fails
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use pumpfun::{PumpFun, common::types::{Cluster, PriorityFee}, utils::CreateTokenMetadata};
    /// # use solana_sdk::{commitment_config::CommitmentConfig, signature::Keypair};
    /// # use std::sync::Arc;
    /// #
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// # let payer = Arc::new(Keypair::new());
    /// # let commitment = CommitmentConfig::confirmed();
    /// # let cluster = Cluster::devnet(commitment, PriorityFee::default());
    /// # let client = PumpFun::new(payer, cluster);
    /// let mint = Keypair::new();
    /// let metadata = CreateTokenMetadata {
    ///     name: "My Token".to_string(),
    ///     symbol: "MYTKN".to_string(),
    ///     description: "A test token created with Pump.fun".to_string(),
    ///     file: "path/to/image.png".to_string(),
    ///     twitter: None,
    ///     telegram: None,
    ///     website: Some("https://example.com".to_string()),
    /// };
    ///
    /// let signature = client.create(mint, metadata, None).await?;
    /// println!("Token created! Signature: {}", signature);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn create(
        &self,
        mint: Keypair,
        metadata: utils::CreateTokenMetadata,
        priority_fee: Option<PriorityFee>,
    ) -> Result<Signature, error::ClientError> {
        // First upload metadata and image to IPFS
        let ipfs: utils::TokenMetadataResponse = utils::create_token_metadata(metadata)
            .await
            .map_err(error::ClientError::UploadMetadataError)?;

        // Add priority fee if provided or default to cluster priority fee
        let priority_fee = priority_fee.unwrap_or(self.cluster.priority_fee);
        let mut instructions = Self::get_priority_fee_instructions(&priority_fee);

        // Add create token instruction
        let create_ix = self.get_create_instruction(&mint, ipfs);
        instructions.push(create_ix);

        // Create and sign transaction
        let transaction = get_transaction(
            self.rpc.clone(),
            self.payer.clone(),
            &instructions,
            Some(&[&mint]),
            #[cfg(feature = "versioned-tx")]
            None,
        )
        .await?;

        // Send and confirm transaction
        let signature = self
            .rpc
            .send_and_confirm_transaction(&transaction)
            .await
            .map_err(error::ClientError::SolanaClientError)?;

        Ok(signature)
    }

    /// Creates a new token and immediately buys an initial amount in a single atomic transaction
    ///
    /// This method combines token creation and an initial purchase into a single atomic transaction.
    /// This is often preferred for new token launches as it:
    /// 1. Creates the token and its bonding curve
    /// 2. Makes an initial purchase to establish liquidity
    /// 3. Guarantees that the creator becomes the first holder
    ///
    /// The entire operation is executed as a single transaction, ensuring atomicity.
    ///
    /// # Arguments
    ///
    /// * `mint` - Keypair for the new token mint account that will be created
    /// * `metadata` - Token metadata including name, symbol, description and image file
    /// * `amount_sol` - Amount of SOL to spend on the initial buy, in lamports (1 SOL = 1,000,000,000 lamports)
    /// * `slippage_basis_points` - Optional maximum acceptable slippage in basis points (1 bp = 0.01%).
    ///   If None, defaults to 500 (5%)
    /// * `priority_fee` - Optional priority fee configuration for compute units. If None, uses the
    ///   default from the cluster configuration
    ///
    /// # Returns
    ///
    /// Returns the transaction signature if successful, or a ClientError if the operation fails
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Metadata upload to IPFS fails
    /// - Account retrieval fails
    /// - Transaction creation fails
    /// - Transaction execution on Solana fails
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use pumpfun::{PumpFun, common::types::{Cluster, PriorityFee}, utils::CreateTokenMetadata};
    /// # use solana_sdk::{commitment_config::CommitmentConfig, native_token::sol_to_lamports, signature::Keypair};
    /// # use std::sync::Arc;
    /// #
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// # let payer = Arc::new(Keypair::new());
    /// # let commitment = CommitmentConfig::confirmed();
    /// # let cluster = Cluster::devnet(commitment, PriorityFee::default());
    /// # let client = PumpFun::new(payer, cluster);
    /// let mint = Keypair::new();
    /// let metadata = CreateTokenMetadata {
    ///     name: "My Token".to_string(),
    ///     symbol: "MYTKN".to_string(),
    ///     description: "A test token created with Pump.fun".to_string(),
    ///     file: "path/to/image.png".to_string(),
    ///     twitter: None,
    ///     telegram: None,
    ///     website: Some("https://example.com".to_string()),
    /// };
    ///
    /// // Create token and buy 0.1 SOL worth with 5% slippage tolerance
    /// let amount_sol = sol_to_lamports(0.1f64); // 0.1 SOL in lamports
    /// let slippage_bps = Some(500); // 5%
    ///
    /// let signature = client.create_and_buy(mint, metadata, amount_sol, slippage_bps, None).await?;
    /// println!("Token created and bought! Signature: {}", signature);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn create_and_buy(
        &self,
        mint: Keypair,
        metadata: utils::CreateTokenMetadata,
        amount_sol: u64,
        slippage_basis_points: Option<u64>,
        priority_fee: Option<PriorityFee>,
    ) -> Result<Signature, error::ClientError> {
        // Upload metadata to IPFS first
        let ipfs: utils::TokenMetadataResponse = utils::create_token_metadata(metadata)
            .await
            .map_err(error::ClientError::UploadMetadataError)?;

        // Add priority fee if provided or default to cluster priority fee
        let priority_fee = priority_fee.unwrap_or(self.cluster.priority_fee);
        let mut instructions = Self::get_priority_fee_instructions(&priority_fee);

        // Add create token instruction
        let create_ix = self.get_create_instruction(&mint, ipfs);
        instructions.push(create_ix);

        // Add buy instruction
        let buy_ix = self
            .get_buy_instructions(mint.pubkey(), amount_sol, slippage_basis_points)
            .await?;
        instructions.extend(buy_ix);

        // Create and sign transaction
        let transaction = get_transaction(
            self.rpc.clone(),
            self.payer.clone(),
            &instructions,
            Some(&[&mint]),
            #[cfg(feature = "versioned-tx")]
            None,
        )
        .await?;

        // Send and confirm transaction
        let signature = self
            .rpc
            .send_and_confirm_transaction(&transaction)
            .await
            .map_err(error::ClientError::SolanaClientError)?;

        Ok(signature)
    }

    /// Buys tokens from a bonding curve by spending SOL
    ///
    /// This method purchases tokens from a bonding curve by providing SOL. The amount of tokens
    /// received is determined by the bonding curve formula for the specific token. As more tokens
    /// are purchased, the price increases according to the curve function.
    ///
    /// The method:
    /// 1. Calculates how many tokens will be received for the given SOL amount
    /// 2. Creates an associated token account for the buyer if needed
    /// 3. Executes the buy transaction with slippage protection
    ///
    /// A portion of the SOL is taken as a fee according to the global configuration.
    ///
    /// # Arguments
    ///
    /// * `mint` - Public key of the token mint to buy
    /// * `amount_sol` - Amount of SOL to spend, in lamports (1 SOL = 1,000,000,000 lamports)
    /// * `slippage_basis_points` - Optional maximum acceptable slippage in basis points (1 bp = 0.01%).
    ///   If None, defaults to 500 (5%)
    /// * `priority_fee` - Optional priority fee configuration for compute units. If None, uses the
    ///   default from the cluster configuration
    ///
    /// # Returns
    ///
    /// Returns the transaction signature if successful, or a ClientError if the operation fails
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The bonding curve account cannot be found
    /// - The buy price calculation fails
    /// - Transaction creation fails
    /// - Transaction execution on Solana fails
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use pumpfun::{PumpFun, common::types::{Cluster, PriorityFee}};
    /// # use solana_sdk::{commitment_config::CommitmentConfig, native_token::sol_to_lamports, pubkey, signature::Keypair};
    /// # use std::sync::Arc;
    /// #
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// # let payer = Arc::new(Keypair::new());
    /// # let commitment = CommitmentConfig::confirmed();
    /// # let cluster = Cluster::devnet(commitment, PriorityFee::default());
    /// # let client = PumpFun::new(payer, cluster);
    /// let token_mint = pubkey!("SoMeTokenM1ntAddr3ssXXXXXXXXXXXXXXXXXXXXXXX");
    ///
    /// // Buy 0.01 SOL worth of tokens with 3% max slippage
    /// let amount_sol = sol_to_lamports(0.01f64); // 0.01 SOL in lamports
    /// let slippage_bps = Some(300); // 3%
    ///
    /// let signature = client.buy(token_mint, amount_sol, slippage_bps, None).await?;
    /// println!("Tokens purchased! Signature: {}", signature);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn buy(
        &self,
        mint: Pubkey,
        amount_sol: u64,
        slippage_basis_points: Option<u64>,
        priority_fee: Option<PriorityFee>,
    ) -> Result<Signature, error::ClientError> {
        // Add priority fee if provided or default to cluster priority fee
        let priority_fee = priority_fee.unwrap_or(self.cluster.priority_fee);
        let mut instructions = Self::get_priority_fee_instructions(&priority_fee);

        // Add buy instruction
        let buy_ix = self
            .get_buy_instructions(mint, amount_sol, slippage_basis_points)
            .await?;
        instructions.extend(buy_ix);

        // Create and sign transaction
        let transaction = get_transaction(
            self.rpc.clone(),
            self.payer.clone(),
            &instructions,
            None,
            #[cfg(feature = "versioned-tx")]
            None,
        )
        .await?;

        // Send and confirm transaction
        let signature = self
            .rpc
            .send_and_confirm_transaction(&transaction)
            .await
            .map_err(error::ClientError::SolanaClientError)?;

        Ok(signature)
    }

    /// Sells tokens back to the bonding curve in exchange for SOL
    ///
    /// This method sells tokens back to the bonding curve, receiving SOL in return. The amount of SOL
    /// received is determined by the bonding curve formula for the specific token. As more tokens
    /// are sold, the price decreases according to the curve function.
    ///
    /// The method:
    /// 1. Determines how many tokens to sell (all tokens or a specific amount)
    /// 2. Calculates how much SOL will be received for the tokens
    /// 3. Executes the sell transaction with slippage protection
    ///
    /// A portion of the SOL is taken as a fee according to the global configuration.
    ///
    /// # Arguments
    ///
    /// * `mint` - Public key of the token mint to sell
    /// * `amount_token` - Optional amount of tokens to sell in base units. If None, sells the entire balance
    /// * `slippage_basis_points` - Optional maximum acceptable slippage in basis points (1 bp = 0.01%).
    ///   If None, defaults to 500 (5%)
    /// * `priority_fee` - Optional priority fee configuration for compute units. If None, uses the
    ///   default from the cluster configuration
    ///
    /// # Returns
    ///
    /// Returns the transaction signature if successful, or a ClientError if the operation fails
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The token account cannot be found
    /// - The bonding curve account cannot be found
    /// - The sell price calculation fails
    /// - Transaction creation fails
    /// - Transaction execution on Solana fails
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use pumpfun::{PumpFun, common::types::{Cluster, PriorityFee}};
    /// # use solana_sdk::{commitment_config::CommitmentConfig, signature::Keypair, pubkey};
    /// # use std::sync::Arc;
    /// #
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// # let payer = Arc::new(Keypair::new());
    /// # let commitment = CommitmentConfig::confirmed();
    /// # let cluster = Cluster::devnet(commitment, PriorityFee::default());
    /// # let client = PumpFun::new(payer, cluster);
    /// let token_mint = pubkey!("SoMeTokenM1ntAddr3ssXXXXXXXXXXXXXXXXXXXXXXX");
    ///
    /// // Sell 1000 tokens with 2% max slippage
    /// let amount_tokens = Some(1000);
    /// let slippage_bps = Some(200); // 2%
    ///
    /// let signature = client.sell(token_mint, amount_tokens, slippage_bps, None).await?;
    /// println!("Tokens sold! Signature: {}", signature);
    ///
    /// // Or sell all tokens with default slippage (5%)
    /// let signature = client.sell(token_mint, None, None, None).await?;
    /// println!("All tokens sold! Signature: {}", signature);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn sell(
        &self,
        mint: Pubkey,
        amount_token: Option<u64>,
        slippage_basis_points: Option<u64>,
        priority_fee: Option<PriorityFee>,
    ) -> Result<Signature, error::ClientError> {
        // Add priority fee if provided or default to cluster priority fee
        let priority_fee = priority_fee.unwrap_or(self.cluster.priority_fee);
        let mut instructions = Self::get_priority_fee_instructions(&priority_fee);

        // Add sell instruction
        let sell_ix = self
            .get_sell_instructions(mint, amount_token, slippage_basis_points)
            .await?;
        instructions.extend(sell_ix);

        // Create and sign transaction
        let transaction = get_transaction(
            self.rpc.clone(),
            self.payer.clone(),
            &instructions,
            None,
            #[cfg(feature = "versioned-tx")]
            None,
        )
        .await?;

        // Send and confirm transaction
        let signature = self
            .rpc
            .send_and_confirm_transaction(&transaction)
            .await
            .map_err(error::ClientError::SolanaClientError)?;

        Ok(signature)
    }

    /// Subscribes to real-time events from the Pump.fun program
    ///
    /// This method establishes a WebSocket connection to the Solana cluster and subscribes
    /// to program log events from the Pump.fun program. It parses the emitted events into
    /// structured data types and delivers them through the provided callback function.
    ///
    /// Event types include:
    /// - `CreateEvent`: Emitted when a new token is created
    /// - `TradeEvent`: Emitted when tokens are bought or sold
    /// - `CompleteEvent`: Emitted when a bonding curve operation completes
    /// - `SetParamsEvent`: Emitted when global parameters are updated
    ///
    /// # Arguments
    ///
    /// * `commitment` - Optional commitment level for the subscription. If None, uses the
    ///   default from the cluster configuration
    /// * `callback` - A function that will be called for each event with the following parameters:
    ///   * `signature`: The transaction signature as a String
    ///   * `event`: The parsed PumpFunEvent if successful, or None if parsing failed
    ///   * `error`: Any error that occurred during parsing, or None if successful
    ///   * `response`: The complete RPC logs response for additional context
    ///
    /// # Returns
    ///
    /// Returns a `Subscription` object that manages the lifecycle of the subscription.
    /// When this object is dropped, the subscription is automatically terminated. If
    /// the subscription cannot be established, returns a ClientError.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The WebSocket connection cannot be established
    /// - The subscription request fails
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use pumpfun::{PumpFun, common::types::{Cluster, PriorityFee}};
    /// # use solana_sdk::{commitment_config::CommitmentConfig, signature::Keypair};
    /// # use std::{sync::Arc, error::Error};
    /// #
    /// # async fn example() -> Result<(), Box<dyn Error>> {
    /// # let payer = Arc::new(Keypair::new());
    /// # let commitment = CommitmentConfig::confirmed();
    /// # let cluster = Cluster::devnet(commitment, PriorityFee::default());
    /// # let client = PumpFun::new(payer, cluster);
    /// #
    /// // Subscribe to token events
    /// let subscription = client.subscribe(None, |signature, event, error, _| {
    ///     match event {
    ///         Some(pumpfun::common::stream::PumpFunEvent::Create(create_event)) => {
    ///             println!("New token created: {} ({})", create_event.name, create_event.symbol);
    ///             println!("Mint address: {}", create_event.mint);
    ///         },
    ///         Some(pumpfun::common::stream::PumpFunEvent::Trade(trade_event)) => {
    ///             let action = if trade_event.is_buy { "bought" } else { "sold" };
    ///             println!(
    ///                 "User {} {} {} tokens for {} SOL",
    ///                 trade_event.user,
    ///                 action,
    ///                 trade_event.token_amount,
    ///                 trade_event.sol_amount as f64 / 1_000_000_000.0
    ///             );
    ///         },
    ///         Some(event) => println!("Other event received: {:#?}", event),
    ///         None => {
    ///             if let Some(err) = error {
    ///                 eprintln!("Error parsing event in tx {}: {}", signature, err);
    ///             }
    ///         }
    ///     }
    /// }).await?;
    ///
    /// // Keep the subscription active
    /// // When no longer needed, drop the subscription to unsubscribe
    /// # Ok(())
    /// # }
    /// ```
    #[cfg(feature = "stream")]
    pub async fn subscribe<F>(
        &self,
        commitment: Option<solana_sdk::commitment_config::CommitmentConfig>,
        callback: F,
    ) -> Result<common::stream::Subscription, error::ClientError>
    where
        F: Fn(
                String,
                Option<common::stream::PumpFunEvent>,
                Option<Box<dyn std::error::Error>>,
                solana_client::rpc_response::Response<solana_client::rpc_response::RpcLogsResponse>,
            ) + Send
            + Sync
            + 'static,
    {
        common::stream::subscribe(self.cluster.clone(), commitment, callback).await
    }

    /// Creates compute budget instructions for priority fees
    ///
    /// Generates Solana compute budget instructions based on the provided priority fee
    /// configuration. These instructions are used to set the maximum compute units a
    /// transaction can consume and the price per compute unit, which helps prioritize
    /// transaction processing during network congestion.
    ///
    /// # Arguments
    ///
    /// * `priority_fee` - Priority fee configuration containing optional unit limit and unit price
    ///
    /// # Returns
    ///
    /// Returns a vector of instructions to set compute budget parameters, which can be
    /// empty if no priority fee parameters are provided
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use pumpfun::{PumpFun, common::types::PriorityFee};
    /// # use solana_sdk::instruction::Instruction;
    /// #
    /// // Set both compute unit limit and price
    /// let priority_fee = PriorityFee {
    ///     unit_limit: Some(200_000),
    ///     unit_price: Some(1_000), // 1000 micro-lamports per compute unit
    /// };
    ///
    /// let compute_instructions: Vec<Instruction> = PumpFun::get_priority_fee_instructions(&priority_fee);
    /// ```
    pub fn get_priority_fee_instructions(priority_fee: &PriorityFee) -> Vec<Instruction> {
        let mut instructions = Vec::new();

        if let Some(limit) = priority_fee.unit_limit {
            let limit_ix = ComputeBudgetInstruction::set_compute_unit_limit(limit);
            instructions.push(limit_ix);
        }

        if let Some(price) = priority_fee.unit_price {
            let price_ix = ComputeBudgetInstruction::set_compute_unit_price(price);
            instructions.push(price_ix);
        }

        instructions
    }

    /// Creates an instruction for initializing a new token
    ///
    /// Generates a Solana instruction to create a new token with a bonding curve on Pump.fun.
    /// This instruction will initialize the token mint, metadata, and bonding curve accounts.
    ///
    /// # Arguments
    ///
    /// * `mint` - Keypair for the new token mint account that will be created
    /// * `ipfs` - Token metadata response from IPFS upload containing name, symbol, and URI
    ///
    /// # Returns
    ///
    /// Returns a Solana instruction for creating a new token
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use pumpfun::{PumpFun, common::types::{Cluster, PriorityFee}, utils};
    /// # use solana_sdk::{commitment_config::CommitmentConfig, signature::Keypair};
    /// # use std::sync::Arc;
    /// #
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// # let payer = Arc::new(Keypair::new());
    /// # let commitment = CommitmentConfig::confirmed();
    /// # let cluster = Cluster::devnet(commitment, PriorityFee::default());
    /// # let client = PumpFun::new(payer, cluster);
    /// #
    /// let mint = Keypair::new();
    /// let metadata_response = utils::create_token_metadata(
    ///     utils::CreateTokenMetadata {
    ///         name: "Example Token".to_string(),
    ///         symbol: "EXTKN".to_string(),
    ///         description: "An example token".to_string(),
    ///         file: "path/to/image.png".to_string(),
    ///         twitter: None,
    ///         telegram: None,
    ///         website: None,
    ///     }
    /// ).await?;
    ///
    /// let create_instruction = client.get_create_instruction(&mint, metadata_response);
    /// # Ok(())
    /// # }
    /// ```
    pub fn get_create_instruction(
        &self,
        mint: &Keypair,
        ipfs: utils::TokenMetadataResponse,
    ) -> Instruction {
        instructions::create(
            &self.payer,
            mint,
            instructions::Create {
                name: ipfs.metadata.name,
                symbol: ipfs.metadata.symbol,
                uri: ipfs.metadata.image,
                creator: self.payer.pubkey(),
            },
        )
    }

    /// Generates instructions for buying tokens from a bonding curve
    ///
    /// Creates a set of Solana instructions needed to purchase tokens using SOL. These
    /// instructions may include creating an associated token account if needed, and the actual
    /// buy instruction with slippage protection.
    ///
    /// # Arguments
    ///
    /// * `mint` - Public key of the token mint to buy
    /// * `amount_sol` - Amount of SOL to spend, in lamports (1 SOL = 1,000,000,000 lamports)
    /// * `slippage_basis_points` - Optional maximum acceptable slippage in basis points (1 bp = 0.01%).
    ///   If None, defaults to 500 (5%)
    ///
    /// # Returns
    ///
    /// Returns a vector of Solana instructions if successful, or a ClientError if the operation fails
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The global account or bonding curve account cannot be fetched
    /// - The buy price calculation fails
    /// - Token account-related operations fail
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use pumpfun::{PumpFun, common::types::{Cluster, PriorityFee}};
    /// # use solana_sdk::{commitment_config::CommitmentConfig, native_token::sol_to_lamports, signature::Keypair, pubkey};
    /// # use std::sync::Arc;
    /// #
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// # let payer = Arc::new(Keypair::new());
    /// # let commitment = CommitmentConfig::confirmed();
    /// # let cluster = Cluster::devnet(commitment, PriorityFee::default());
    /// # let client = PumpFun::new(payer, cluster);
    /// #
    /// let mint = pubkey!("TokenM1ntPubk3yXXXXXXXXXXXXXXXXXXXXXXXXXXXX");
    /// let amount_sol = sol_to_lamports(0.01); // 0.01 SOL
    /// let slippage_bps = Some(300); // 3%
    ///
    /// let buy_instructions = client.get_buy_instructions(mint, amount_sol, slippage_bps).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn get_buy_instructions(
        &self,
        mint: Pubkey,
        amount_sol: u64,
        slippage_basis_points: Option<u64>,
    ) -> Result<Vec<Instruction>, error::ClientError> {
        // Get accounts and calculate buy amounts
        let global_account = self.get_global_account().await?;
        let buy_amount = {
            let bonding_curve_pda = Self::get_bonding_curve_pda(&mint)
                .ok_or(error::ClientError::BondingCurveNotFound)?;
            if self.rpc.get_account(&bonding_curve_pda).await.is_err() {
                global_account.get_initial_buy_price(amount_sol)
            } else {
                let bonding_curve_account = self.get_bonding_curve_account(&mint).await?;
                bonding_curve_account
                    .get_buy_price(amount_sol)
                    .map_err(error::ClientError::BondingCurveError)?
            }
        };
        let buy_amount_with_slippage =
            utils::calculate_with_slippage_buy(amount_sol, slippage_basis_points.unwrap_or(500));

        let mut instructions = Vec::new();

        // Create Associated Token Account if needed
        #[cfg(feature = "create-ata")]
        {
            let ata: Pubkey = get_associated_token_address(&self.payer.pubkey(), &mint);
            if self.rpc.get_account(&ata).await.is_err() {
                instructions.push(create_associated_token_account(
                    &self.payer.pubkey(),
                    &self.payer.pubkey(),
                    &mint,
                    &constants::accounts::TOKEN_PROGRAM,
                ));
            }
        }

        // Add buy instruction
        instructions.push(instructions::buy(
            &self.payer,
            &mint,
            &global_account.fee_recipient,
            &self.payer.pubkey(),
            instructions::Buy {
                amount: buy_amount,
                max_sol_cost: buy_amount_with_slippage,
            },
        ));

        Ok(instructions)
    }

    /// Generates instructions for selling tokens back to a bonding curve
    ///
    /// Creates a set of Solana instructions needed to sell tokens in exchange for SOL. These
    /// instructions include the sell instruction with slippage protection and may include
    /// closing the associated token account if all tokens are being sold and the feature
    /// is enabled.
    ///
    /// # Arguments
    ///
    /// * `mint` - Public key of the token mint to sell
    /// * `amount_token` - Optional amount of tokens to sell in base units. If None, sells the entire balance
    /// * `slippage_basis_points` - Optional maximum acceptable slippage in basis points (1 bp = 0.01%).
    ///   If None, defaults to 500 (5%)
    ///
    /// # Returns
    ///
    /// Returns a vector of Solana instructions if successful, or a ClientError if the operation fails
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The token account or token balance cannot be fetched
    /// - The global account or bonding curve account cannot be fetched
    /// - The sell price calculation fails
    /// - Token account closing operations fail (when applicable)
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use pumpfun::{PumpFun, common::types::{Cluster, PriorityFee}};
    /// # use solana_sdk::{commitment_config::CommitmentConfig, signature::Keypair, pubkey};
    /// # use std::sync::Arc;
    /// #
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// # let payer = Arc::new(Keypair::new());
    /// # let commitment = CommitmentConfig::confirmed();
    /// # let cluster = Cluster::devnet(commitment, PriorityFee::default());
    /// # let client = PumpFun::new(payer, cluster);
    /// #
    /// let mint = pubkey!("TokenM1ntPubk3yXXXXXXXXXXXXXXXXXXXXXXXXXXXX");
    /// let amount_tokens = Some(1000); // Sell 1000 tokens
    /// let slippage_bps = Some(200); // 2%
    ///
    /// let sell_instructions = client.get_sell_instructions(mint, amount_tokens, slippage_bps).await?;
    ///
    /// // Or to sell all tokens:
    /// let sell_all_instructions = client.get_sell_instructions(mint, None, None).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn get_sell_instructions(
        &self,
        mint: Pubkey,
        amount_token: Option<u64>,
        slippage_basis_points: Option<u64>,
    ) -> Result<Vec<Instruction>, error::ClientError> {
        // Get ATA
        let ata: Pubkey = get_associated_token_address(&self.payer.pubkey(), &mint);

        // Get token balance
        let token_balance = if amount_token.is_none() || cfg!(feature = "close-ata") {
            // We need the balance if amount_token is None OR if the close-ata feature is enabled
            let balance = self.rpc.get_token_account_balance(&ata).await?;
            Some(balance.amount.parse::<u64>().unwrap())
        } else {
            None
        };

        // Determine amount to sell
        let amount = amount_token.unwrap_or_else(|| token_balance.unwrap());

        // Calculate min sol output
        let global_account = self.get_global_account().await?;
        let bonding_curve_account = self.get_bonding_curve_account(&mint).await?;
        let min_sol_output = bonding_curve_account
            .get_sell_price(amount, global_account.fee_basis_points)
            .map_err(error::ClientError::BondingCurveError)?;
        let min_sol_output = utils::calculate_with_slippage_sell(
            min_sol_output,
            slippage_basis_points.unwrap_or(500),
        );

        let mut instructions = Vec::new();

        // Add sell instruction
        instructions.push(instructions::sell(
            &self.payer,
            &mint,
            &global_account.fee_recipient,
            &bonding_curve_account.creator,
            instructions::Sell {
                amount,
                min_sol_output,
            },
        ));

        // Close account if balance equals amount
        #[cfg(feature = "close-ata")]
        {
            // Token balance should be guaranteed to be available at this point
            // due to our fetch logic in the beginning of the function
            if let Some(balance) = token_balance {
                // Only close the account if we're selling all tokens
                if balance == amount {
                    let token_program = constants::accounts::TOKEN_PROGRAM;

                    // Verify the token account exists before attempting to close it
                    if self.rpc.get_account(&ata).await.is_ok() {
                        // Create instruction to close the ATA
                        let close_instruction = close_account(
                            &token_program,
                            &ata,
                            &self.payer.pubkey(),
                            &self.payer.pubkey(),
                            &[&self.payer.pubkey()],
                        )
                        .map_err(|err| {
                            error::ClientError::OtherError(format!(
                                "Failed to create close account instruction: pubkey={}: {}",
                                ata, err
                            ))
                        })?;

                        instructions.push(close_instruction);
                    } else {
                        // Log warning but don't fail the transaction if account doesn't exist
                        eprintln!(
                            "Warning: Cannot close token account {}, it doesn't exist",
                            ata
                        );
                    }
                }
            } else {
                // This case should not occur due to our balance fetch logic,
                // but handle it gracefully just in case
                eprintln!("Warning: Token balance unavailable, not closing account");
            }
        }

        Ok(instructions)
    }

    /// Gets the Program Derived Address (PDA) for the global state account
    ///
    /// Derives the address of the global state account using the program ID and a
    /// constant seed. The global state account contains program-wide configuration
    /// such as fee settings and fee recipient.
    ///
    /// # Returns
    ///
    /// Returns the PDA public key derived from the GLOBAL_SEED
    ///
    /// # Examples
    ///
    /// ```
    /// # use pumpfun::PumpFun;
    /// # use solana_sdk::pubkey::Pubkey;
    /// #
    /// let global_pda: Pubkey = PumpFun::get_global_pda();
    /// println!("Global state account: {}", global_pda);
    /// ```
    pub fn get_global_pda() -> Pubkey {
        let seeds: &[&[u8]; 1] = &[constants::seeds::GLOBAL_SEED];
        let program_id: &Pubkey = &constants::accounts::PUMPFUN;
        Pubkey::find_program_address(seeds, program_id).0
    }

    /// Gets the Program Derived Address (PDA) for the mint authority
    ///
    /// Derives the address of the mint authority PDA using the program ID and a
    /// constant seed. The mint authority PDA is the authority that can mint new
    /// tokens for any token created through the Pump.fun program.
    ///
    /// # Returns
    ///
    /// Returns the PDA public key derived from the MINT_AUTHORITY_SEED
    ///
    /// # Examples
    ///
    /// ```
    /// # use pumpfun::PumpFun;
    /// # use solana_sdk::pubkey::Pubkey;
    /// #
    /// let mint_authority: Pubkey = PumpFun::get_mint_authority_pda();
    /// println!("Mint authority account: {}", mint_authority);
    /// ```
    pub fn get_mint_authority_pda() -> Pubkey {
        let seeds: &[&[u8]; 1] = &[constants::seeds::MINT_AUTHORITY_SEED];
        let program_id: &Pubkey = &constants::accounts::PUMPFUN;
        Pubkey::find_program_address(seeds, program_id).0
    }

    /// Gets the Program Derived Address (PDA) for a token's bonding curve account
    ///
    /// Derives the address of a token's bonding curve account using the program ID,
    /// a constant seed, and the token mint address. The bonding curve account stores
    /// the state and parameters that govern the token's price dynamics.
    ///
    /// # Arguments
    ///
    /// * `mint` - Public key of the token mint
    ///
    /// # Returns
    ///
    /// Returns Some(PDA) if derivation succeeds, or None if it fails
    ///
    /// # Examples
    ///
    /// ```
    /// # use pumpfun::PumpFun;
    /// # use solana_sdk::{pubkey, pubkey::Pubkey};
    /// #
    /// let mint = pubkey!("TokenM1ntPubk3yXXXXXXXXXXXXXXXXXXXXXXXXXXXX");
    /// if let Some(bonding_curve) = PumpFun::get_bonding_curve_pda(&mint) {
    ///     println!("Bonding curve account: {}", bonding_curve);
    /// }
    /// ```
    pub fn get_bonding_curve_pda(mint: &Pubkey) -> Option<Pubkey> {
        let seeds: &[&[u8]; 2] = &[constants::seeds::BONDING_CURVE_SEED, mint.as_ref()];
        let program_id: &Pubkey = &constants::accounts::PUMPFUN;
        let pda: Option<(Pubkey, u8)> = Pubkey::try_find_program_address(seeds, program_id);
        pda.map(|pubkey| pubkey.0)
    }

    /// Gets the Program Derived Address (PDA) for a token's metadata account
    ///
    /// Derives the address of a token's metadata account following the Metaplex Token Metadata
    /// standard. The metadata account stores information about the token such as name,
    /// symbol, and URI pointing to additional metadata.
    ///
    /// # Arguments
    ///
    /// * `mint` - Public key of the token mint
    ///
    /// # Returns
    ///
    /// Returns the PDA public key for the token's metadata account
    ///
    /// # Examples
    ///
    /// ```
    /// # use pumpfun::PumpFun;
    /// # use solana_sdk::{pubkey, pubkey::Pubkey};
    /// #
    /// let mint = pubkey!("TokenM1ntPubk3yXXXXXXXXXXXXXXXXXXXXXXXXXXXX");
    /// let metadata_pda = PumpFun::get_metadata_pda(&mint);
    /// println!("Token metadata account: {}", metadata_pda);
    /// ```
    pub fn get_metadata_pda(mint: &Pubkey) -> Pubkey {
        let seeds: &[&[u8]; 3] = &[
            constants::seeds::METADATA_SEED,
            constants::accounts::MPL_TOKEN_METADATA.as_ref(),
            mint.as_ref(),
        ];
        let program_id: &Pubkey = &constants::accounts::MPL_TOKEN_METADATA;
        Pubkey::find_program_address(seeds, program_id).0
    }

    /// Gets the global state account data containing program-wide configuration
    ///
    /// Fetches and deserializes the global state account which contains program-wide
    /// configuration parameters such as:
    /// - Fee basis points for trading
    /// - Fee recipient account
    /// - Bonding curve parameters
    /// - Other platform-wide settings
    ///
    /// # Returns
    ///
    /// Returns the deserialized GlobalAccount if successful, or a ClientError if the operation fails
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The account cannot be found on-chain
    /// - The account data cannot be properly deserialized
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use pumpfun::{PumpFun, common::types::{Cluster, PriorityFee}};
    /// # use solana_sdk::{commitment_config::CommitmentConfig, signature::Keypair};
    /// # use std::sync::Arc;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// # let payer = Arc::new(Keypair::new());
    /// # let commitment = CommitmentConfig::confirmed();
    /// # let cluster = Cluster::devnet(commitment, PriorityFee::default());
    /// # let client = PumpFun::new(payer, cluster);
    /// let global = client.get_global_account().await?;
    /// println!("Fee basis points: {}", global.fee_basis_points);
    /// println!("Fee recipient: {}", global.fee_recipient);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn get_global_account(&self) -> Result<accounts::GlobalAccount, error::ClientError> {
        let global: Pubkey = Self::get_global_pda();

        let account = self
            .rpc
            .get_account(&global)
            .await
            .map_err(error::ClientError::SolanaClientError)?;

        solana_sdk::borsh1::try_from_slice_unchecked::<accounts::GlobalAccount>(&account.data)
            .map_err(error::ClientError::BorshError)
    }

    /// Gets a token's bonding curve account data containing pricing parameters
    ///
    /// Fetches and deserializes a token's bonding curve account which contains the
    /// state and parameters that determine the token's price dynamics, including:
    /// - Current supply
    /// - Reserve balance
    /// - Bonding curve parameters
    /// - Other token-specific configuration
    ///
    /// # Arguments
    ///
    /// * `mint` - Public key of the token mint
    ///
    /// # Returns
    ///
    /// Returns the deserialized BondingCurveAccount if successful, or a ClientError if the operation fails
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The bonding curve PDA cannot be derived
    /// - The account cannot be found on-chain
    /// - The account data cannot be properly deserialized
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use pumpfun::{PumpFun, common::types::{Cluster, PriorityFee}};
    /// # use solana_sdk::{commitment_config::CommitmentConfig, signature::Keypair, pubkey};
    /// # use std::sync::Arc;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// # let payer = Arc::new(Keypair::new());
    /// # let commitment = CommitmentConfig::confirmed();
    /// # let cluster = Cluster::devnet(commitment, PriorityFee::default());
    /// # let client = PumpFun::new(payer, cluster);
    /// let mint = pubkey!("TokenM1ntPubk3yXXXXXXXXXXXXXXXXXXXXXXXXXXXX");
    /// let bonding_curve = client.get_bonding_curve_account(&mint).await?;
    /// println!("Bonding Curve Account: {:#?}", bonding_curve);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn get_bonding_curve_account(
        &self,
        mint: &Pubkey,
    ) -> Result<accounts::BondingCurveAccount, error::ClientError> {
        let bonding_curve_pda =
            Self::get_bonding_curve_pda(mint).ok_or(error::ClientError::BondingCurveNotFound)?;

        let account = self
            .rpc
            .get_account(&bonding_curve_pda)
            .await
            .map_err(error::ClientError::SolanaClientError)?;

        solana_sdk::borsh1::try_from_slice_unchecked::<accounts::BondingCurveAccount>(&account.data)
            .map_err(error::ClientError::BorshError)
    }

    /// Gets the creator vault address (for claiming pump creator fees)
    ///
    /// Derives the token creator's vault using the program ID,
    /// a constant seed, and the creator's address.
    ///
    /// # Arguments
    ///
    /// * `creator` - Public key of the token's creator
    ///
    /// # Returns
    ///
    /// Returns Some(PDA) if derivation succeeds, or None if it fails
    ///
    /// # Examples
    ///
    /// ```
    /// # use pumpfun::PumpFun;
    /// # use solana_sdk::{pubkey, pubkey::Pubkey};
    /// #
    /// let creator = pubkey!("Amya8kr2bzEY9kyXXXXXXXXXXXXXXXXXXXXXXXXXXXX");
    /// if let Some(bonding_curve) = PumpFun::get_creator_vault_pda(&creator) {
    ///     println!("Creator vault address: {}", creator);
    /// }
    /// ```
    pub fn get_creator_vault_pda(creator: &Pubkey) -> Option<Pubkey> {
        let seeds: &[&[u8]; 2] = &[constants::seeds::CREATOR_VAULT_SEED, creator.as_ref()];
        let program_id: &Pubkey = &constants::accounts::PUMPFUN;
        let pda: Option<(Pubkey, u8)> = Pubkey::try_find_program_address(seeds, program_id);
        pda.map(|pubkey| pubkey.0)
    }

    /// Returns the PDA of a user volume accumulator account.
    ///
    /// # Arguments
    /// * `user` - Public key of the user.
    ///
    /// # Returns
    /// PDA of the corresponding user volume accumulator account.
    pub fn get_user_volume_accumulator_pda(user: &Pubkey) -> Pubkey {
        let (user_volume_accumulator, _bump) = Pubkey::find_program_address(
            &[b"user_volume_accumulator", user.as_ref()],
            &constants::accounts::PUMPFUN,
        );
        user_volume_accumulator
    }
}

use std::sync::Arc;

use solana_client::{nonblocking::rpc_client::RpcClient, rpc_client::SerializableTransaction};
#[cfg(not(feature = "versioned-tx"))]
use solana_sdk::transaction::Transaction;
use solana_sdk::{instruction::Instruction, signature::Keypair, signer::Signer, hash::Hash};
#[cfg(feature = "versioned-tx")]
use solana_sdk::{
    message::{v0, AddressLookupTableAccount, VersionedMessage},
    transaction::VersionedTransaction,
};

use crate::error;

/// Constructs a signed transaction from a set of instructions and signers
///
/// This method creates a transaction with the provided instructions and signers,
/// obtaining a recent blockhash from the Solana network. It handles the process
/// of creating a properly formed transaction that can be submitted to the network.
///
/// # Arguments
///
/// * `rpc` - An Arc-wrapped RpcClient used to fetch the recent blockhash
/// * `payer` - The primary account that will pay for the transaction fees
/// * `instructions` - Slice of Solana instructions to include in the transaction
/// * `additional_signers` - Optional slice of additional keypair signers that should sign the transaction,
///   in addition to the payer
/// * `address_lookup_table_accounts` - Optional slice of Address Lookup Table accounts to include,
///   enabling versioned transactions with address table lookups
///   (only available with "versioned-tx" feature)
///
/// # Returns
///
/// Returns a signed Transaction (or VersionedTransaction when the "versioned-tx" feature is enabled)
/// if successful, or a ClientError if the operation fails
///
/// # Errors
///
/// Returns an error if:
/// - Failed to retrieve the recent blockhash from the network
/// - Transaction creation fails due to invalid parameters
/// - Transaction message compilation fails (for versioned transactions)
/// - Transaction signing fails
///
/// # Feature flags
///
/// When compiled with the "versioned-tx" feature, this function returns a VersionedTransaction
/// that supports Address Lookup Tables. Otherwise, it returns a standard Transaction.
///
/// # Examples
///
/// ```no_run
/// # use pumpfun::{
/// #     common::types::{Cluster, PriorityFee},
/// #     utils::transaction::get_transaction,
/// #     PumpFun,
/// # };
/// # use solana_sdk::{
/// #     commitment_config::CommitmentConfig, instruction::Instruction,
/// #     message::AddressLookupTableAccount, signature::Keypair,
/// # };
/// # use std::sync::Arc;
/// #
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// # let payer = Arc::new(Keypair::new());
/// # let cluster = Cluster::devnet(CommitmentConfig::confirmed(), PriorityFee::default());
/// # let client = PumpFun::new(payer, cluster);
/// # let instructions: Vec<Instruction> = Vec::new();
/// # let custom_signer = Keypair::new();
/// #
/// // Create a transaction with multiple signers
/// let transaction = get_transaction(
///     client.rpc.clone(),
///     client.payer.clone(),
///     &instructions,
///     Some(&[&custom_signer]),
/// #   #[cfg(feature = "versioned-tx")]
/// #   None,
/// )
/// .await?;
///
/// // Or with just the payer as signer
/// let transaction = get_transaction(
///     client.rpc.clone(),
///     client.payer.clone(),
///     &instructions,
///     None,
/// #   #[cfg(feature = "versioned-tx")]
/// #   None,
/// )
/// .await?;
///
/// // Create a versioned transaction with address lookup tables (when "versioned-tx" feature is enabled)
/// let lookup_tables: Vec<AddressLookupTableAccount> = Vec::new();
/// let transaction = get_transaction(
///     client.rpc.clone(),
///     client.payer.clone(),
///     &instructions,
///     None,
/// #   #[cfg(feature = "versioned-tx")]
///     Some(&lookup_tables),
/// )
/// .await?;
/// Ok(())
/// # }
/// ```
pub async fn get_transaction(
    rpc: Arc<RpcClient>,
    payer: Arc<Keypair>,
    instructions: &[Instruction],
    additional_signers: Option<&[&Keypair]>,
    #[cfg(feature = "versioned-tx")] address_lookup_table_accounts: Option<
        &[AddressLookupTableAccount],
    >,
) -> Result<impl SerializableTransaction, error::ClientError> {
    // Get recent blockhash for transaction validity window
    let recent_blockhash = rpc
        .get_latest_blockhash()
        .await
        .map_err(error::ClientError::SolanaClientError)?;

    // Create a combined signers array with payer and additional signers
    let mut all_signers =
        Vec::with_capacity(1 + additional_signers.map_or(0, |signers| signers.len()));
    all_signers.push(&*payer);

    if let Some(signers) = additional_signers {
        all_signers.extend(signers);
    }

    // Create and sign legacy transaction with all signers
    #[cfg(not(feature = "versioned-tx"))]
    let transaction = Transaction::new_signed_with_payer(
        instructions,
        Some(&payer.pubkey()),
        &all_signers,
        recent_blockhash,
    );

    // Create and sign versioned transaction with all signers
    #[cfg(feature = "versioned-tx")]
    let transaction = {
        let message = match v0::Message::try_compile(
            &payer.pubkey(),
            instructions,
            address_lookup_table_accounts.unwrap_or(&[]),
            recent_blockhash,
        ) {
            Ok(msg) => VersionedMessage::V0(msg),
            Err(e) => {
                return Err(error::ClientError::OtherError(format!(
                    "Failed to compile transaction message: {}",
                    e
                )))
            }
        };

        match VersionedTransaction::try_new(message, &all_signers) {
            Ok(tx) => tx,
            Err(e) => {
                return Err(error::ClientError::OtherError(format!(
                    "Failed to sign transaction: {}",
                    e
                )))
            }
        }
    };

    Ok(transaction)
}

pub fn get_transaction_offline_prepared(
    recent_blockhash: &Hash,
    _rpc: Arc<RpcClient>,
    payer: Arc<Keypair>,
    instructions: &[Instruction],
    additional_signers: Option<&[&Keypair]>,
    #[cfg(feature = "versioned-tx")] address_lookup_table_accounts: Option<
        &[AddressLookupTableAccount],
    >,
) -> Result<impl SerializableTransaction, error::ClientError>  {
    // Create a combined signers array with payer and additional signers
    let mut all_signers =
        Vec::with_capacity(1 + additional_signers.map_or(0, |signers| signers.len()));
    all_signers.push(&*payer);

    if let Some(signers) = additional_signers {
        all_signers.extend(signers);
    }

    // Create and sign legacy transaction with all signers
    #[cfg(not(feature = "versioned-tx"))]
    let transaction = Transaction::new_signed_with_payer(
        instructions,
        Some(&payer.pubkey()),
        &all_signers,
        *recent_blockhash,
    );

    // Create and sign versioned transaction with all signers
    #[cfg(feature = "versioned-tx")]
    let transaction = {
        let message = match v0::Message::try_compile(
            &payer.pubkey(),
            instructions,
            address_lookup_table_accounts.unwrap_or(&[]),
            *recent_blockhash,
        ) {
            Ok(msg) => VersionedMessage::V0(msg),
            Err(e) => {
                return Err(error::ClientError::OtherError(format!(
                    "Failed to compile transaction message: {}",
                    e
                )))
            }
        };

        match VersionedTransaction::try_new(message, &all_signers) {
            Ok(tx) => tx,
            Err(e) => {
                return Err(error::ClientError::OtherError(format!(
                    "Failed to sign transaction: {}",
                    e
                )))
            }
        }
    };

    Ok(transaction)
}

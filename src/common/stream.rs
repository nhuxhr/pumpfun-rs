use std::error::Error;

use base64::Engine;
use borsh::{BorshDeserialize, BorshSerialize};
use futures::StreamExt;
use serde::{Deserialize, Serialize};
use solana_client::{
    nonblocking::pubsub_client::PubsubClient,
    rpc_config::{RpcTransactionLogsConfig, RpcTransactionLogsFilter},
    rpc_response::{Response, RpcLogsResponse},
};
use solana_sdk::{commitment_config::CommitmentConfig, pubkey::Pubkey};
use tokio::sync::mpsc;
use tokio::task::JoinHandle;

use super::types::Cluster;
use crate::{constants, error};

/// Event emitted when a new token is created
///
/// This event contains information about a newly created token, including its
/// metadata, mint address, bonding curve address, and the accounts involved.
#[derive(BorshSerialize, BorshDeserialize, Debug, Serialize, Deserialize, Clone)]
pub struct CreateEvent {
    pub name: String,
    pub symbol: String,
    pub uri: String,
    pub mint: Pubkey,
    pub bonding_curve: Pubkey,
    pub user: Pubkey,
    pub creator: Pubkey,
    pub timestamp: i64,
    pub virtual_token_reserves: u64,
    pub virtual_sol_reserves: u64,
    pub real_token_reserves: u64,
    pub token_total_supply: u64,
}

/// Event emitted when a token is bought or sold
///
/// This event contains details about a trade transaction, including the amounts
/// exchanged, the type of trade (buy/sell), and the updated bonding curve state.
#[derive(BorshSerialize, BorshDeserialize, Debug, Serialize, Deserialize, Clone)]
pub struct TradeEvent {
    pub mint: Pubkey,
    pub sol_amount: u64,
    pub token_amount: u64,
    pub is_buy: bool,
    pub user: Pubkey,
    pub timestamp: i64,
    pub virtual_sol_reserves: u64,
    pub virtual_token_reserves: u64,
    pub real_sol_reserves: u64,
    pub real_token_reserves: u64,
    pub fee_recipient: Pubkey,
    pub fee_basis_points: u64,
    pub fee: u64,
    pub creator: Pubkey,
    pub creator_fee_basis_points: u64,
    pub creator_fee: u64,
    pub track_volume: bool,
    pub total_unclaimed_tokens: u64,
    pub total_claimed_tokens: u64,
    pub current_sol_volume: u64,
    pub last_update_timestamp: i64,
}

/// Event emitted when a bonding curve operation completes
///
/// This event signals the completion of a bonding curve operation,
/// providing information about the involved accounts.
#[derive(BorshSerialize, BorshDeserialize, Debug, Serialize, Deserialize, Clone)]
pub struct CompleteEvent {
    pub user: Pubkey,
    pub mint: Pubkey,
    pub bonding_curve: Pubkey,
    pub timestamp: i64,
}

/// Event emitted when global parameters are updated
///
/// This event contains information about updates to the global program parameters,
/// including fee settings and initial bonding curve configuration values.
#[derive(BorshSerialize, BorshDeserialize, Debug, Serialize, Deserialize, Clone)]
pub struct SetParamsEvent {
    pub initial_virtual_token_reserves: u64,
    pub initial_virtual_sol_reserves: u64,
    pub initial_real_token_reserves: u64,
    pub final_real_sol_reserves: u64,
    pub token_total_supply: u64,
    pub fee_basis_points: u64,
    pub withdraw_authority: Pubkey,
    pub enable_migrate: bool,
    pub pool_migration_fee: u64,
    pub creator_fee_basis_points: u64,
    pub fee_recipients: [Pubkey; 8],
    pub timestamp: i64,
    pub set_creator_authority: Pubkey,
    pub admin_set_creator_authority: Pubkey,
}

/// Enum representing all possible event types emitted by the Pump.fun program
///
/// This enum acts as a container for the different event types that can be
/// emitted by the program. It's used to provide a unified type for event handlers.
#[derive(Debug, Serialize, Deserialize)]
pub enum PumpFunEvent {
    Create(CreateEvent),
    Trade(TradeEvent),
    Complete(CompleteEvent),
    SetParams(SetParamsEvent),
    Unhandled(String, Vec<u8>), // For unhandled events
    Unknown(String, Vec<u8>),   // For unknown events
}

/// Represents an active WebSocket subscription to Pump.fun events
///
/// This struct manages the lifecycle of an event subscription, automatically
/// unsubscribing when dropped to ensure proper cleanup of resources.
pub struct Subscription {
    pub task: JoinHandle<()>,
    pub unsubscribe: Box<dyn Fn() + Send>,
}

impl Subscription {
    pub fn new(task: JoinHandle<()>, unsubscribe: Box<dyn Fn() + Send>) -> Self {
        Subscription { task, unsubscribe }
    }
}

impl Drop for Subscription {
    fn drop(&mut self) {
        (self.unsubscribe)();
        self.task.abort();
    }
}

/// Parses base64-encoded program log data into a structured PumpFunEvent
///
/// This function decodes the base64 data from program logs, identifies the event type
/// using the discriminator (first 8 bytes), and deserializes the remaining data into
/// the appropriate event structure.
///
/// # Arguments
///
/// * `signature` - Transaction signature associated with the event
/// * `data` - Base64-encoded event data from program logs
///
/// # Returns
///
/// Returns a parsed PumpFunEvent if successful, or an error if parsing fails
pub fn parse_event(
    signature: &str,
    data: &str,
) -> Result<PumpFunEvent, Box<dyn Error + Send + Sync>> {
    // Decode base64
    let decoded = base64::engine::general_purpose::STANDARD.decode(data)?;

    // Get event type from the first 8 bytes
    if decoded.len() < 8 {
        return Err(format!("Data too short to contain discriminator: {}", data).into());
    }

    let discriminator = &decoded[..8];
    match discriminator {
        // CreateEvent
        [27, 114, 169, 77, 222, 235, 99, 118] => Ok(PumpFunEvent::Create(
            CreateEvent::try_from_slice(&decoded[8..])
                .map_err(|e| format!("Failed to decode CreateEvent: {}", e))?,
        )),
        // TradeEvent
        [189, 219, 127, 211, 78, 230, 97, 238] => Ok(PumpFunEvent::Trade(
            TradeEvent::try_from_slice(&decoded[8..])
                .map_err(|e| format!("Failed to decode TradeEvent: {}", e))?,
        )),
        // CompleteEvent
        [95, 114, 97, 156, 212, 46, 152, 8] => Ok(PumpFunEvent::Complete(
            CompleteEvent::try_from_slice(&decoded[8..])
                .map_err(|e| format!("Failed to decode CompleteEvent: {}", e))?,
        )),
        // SetParamsEvent
        [223, 195, 159, 246, 62, 48, 143, 131] => Ok(PumpFunEvent::SetParams(
            SetParamsEvent::try_from_slice(&decoded[8..])
                .map_err(|e| format!("Failed to decode SetParamsEvent: {}", e))?,
        )),
        // Other unhandled Pump.fun events
        [64, 69, 192, 104, 29, 30, 25, 107]
        | [245, 59, 70, 34, 75, 185, 109, 92]
        | [147, 250, 108, 120, 247, 29, 67, 222]
        | [79, 172, 246, 49, 205, 91, 206, 232]
        | [146, 159, 189, 172, 146, 88, 56, 244]
        | [122, 2, 127, 1, 14, 191, 12, 175]
        | [189, 233, 93, 185, 92, 148, 234, 148]
        | [97, 97, 215, 144, 93, 146, 22, 124]
        | [134, 36, 13, 72, 232, 101, 130, 216]
        | [237, 52, 123, 37, 245, 251, 72, 210]
        | [142, 203, 6, 32, 127, 105, 191, 162]
        | [197, 122, 167, 124, 116, 81, 91, 255]
        | [182, 195, 137, 42, 35, 206, 207, 247] => {
            Ok(PumpFunEvent::Unhandled(signature.to_string(), decoded))
        }
        // Unknown event type
        _ => Ok(PumpFunEvent::Unknown(signature.to_string(), decoded)),
    }
}

/// Subscribes to Pump.fun program events emitted on-chain
///
/// This function establishes a WebSocket connection to the Solana cluster and
/// subscribes to all transaction logs that mention the Pump.fun program. It parses
/// the program data from these logs into strongly-typed event structures.
///
/// Events are delivered through the provided callback function as they occur. The
/// subscription continues until the returned `Subscription` object is dropped.
///
/// # Arguments
///
/// * `cluster` - Solana cluster configuration containing RPC endpoints
/// * `mentioned` - Optional public key to filter events by mentions. If None, subscribes to all Pump.fun events
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
/// use pumpfun::{PumpFun, common::types::{Cluster, PriorityFee}};
/// use solana_sdk::commitment_config::CommitmentConfig;
/// use std::{sync::Arc, error::Error};
///
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn Error>> {
///     // Create cluster configuration
///     let cluster = Cluster::mainnet(
///         CommitmentConfig::confirmed(),
///         PriorityFee::default()
///     );
///
///     // Define callback to process events
///     let callback = |signature, event, error, _| {
///         if let Some(event) = event {
///             println!("Event received: {:#?} in tx: {}", event, signature);
///         } else if let Some(err) = error {
///             eprintln!("Error parsing event in tx {}: {}", signature, err);
///         }
///     };
///
///     // Subscribe to events
///     let subscription = pumpfun::common::stream::subscribe(cluster, None, None, callback).await?;
///
///     // Keep subscription alive until program terminates
///     tokio::signal::ctrl_c().await?;
///     Ok(())
/// }
/// ```
pub async fn subscribe<F>(
    cluster: Cluster,
    mentioned: Option<String>,
    commitment: Option<CommitmentConfig>,
    callback: F,
) -> Result<Subscription, error::ClientError>
where
    F: Fn(
            String,
            Option<PumpFunEvent>,
            Option<Box<dyn Error + Send + Sync>>,
            Response<RpcLogsResponse>,
        ) + Send
        + Sync
        + 'static,
{
    // Initialize PubsubClient
    let ws_url = &cluster.rpc.ws;
    let pubsub_client = PubsubClient::new(ws_url)
        .await
        .map_err(error::ClientError::PubsubClientError)?;

    let (tx, _) = mpsc::channel(1);
    let (cb_tx, mut cb_rx) = mpsc::channel(1000);

    tokio::spawn(async move {
        while let Some((sig, event, err, log)) = cb_rx.recv().await {
            callback(sig, event, err, log);
        }
    });

    let task = tokio::spawn(async move {
        // Subscribe to logs for the program
        let (mut stream, _unsubscribe) = pubsub_client
            .logs_subscribe(
                RpcTransactionLogsFilter::Mentions(vec![
                    mentioned.unwrap_or(constants::accounts::PUMPFUN.to_string())
                ]),
                RpcTransactionLogsConfig {
                    commitment: Some(commitment.unwrap_or(cluster.commitment)),
                },
            )
            .await
            .unwrap();

        // Process incoming logs
        while let Some(log) = stream.next().await {
            // Get the signature of the transaction
            let signature = &log.value.signature;
            // Check for logs with "Program data:" prefix
            for log_line in &log.value.logs {
                // Extract base64-encoded data
                if let Some(data) = log_line.strip_prefix("Program data: ") {
                    match parse_event(signature, data) {
                        Ok(event) => {
                            let _ = cb_tx
                                .send((signature.to_string(), Some(event), None, log.clone()))
                                .await;
                        }
                        Err(err) => {
                            let _ = cb_tx
                                .send((signature.to_string(), None, Some(err), log.clone()))
                                .await;
                        }
                    }
                }
            }
        }
    });

    Ok(Subscription::new(
        task,
        Box::new(move || {
            let _ = tx.try_send(());
        }),
    ))
}

#[cfg(test)]
mod tests {
    use crate::common::types::PriorityFee;

    use super::*;
    use std::sync::Arc;
    use tokio::sync::Mutex;
    use tokio::time::{timeout, Duration};

    #[cfg(not(skip_expensive_tests))]
    #[tokio::test]
    async fn test_subscribe() {
        if std::env::var("SKIP_EXPENSIVE_TESTS").is_ok() {
            return;
        }

        // Define the cluster
        let cluster = Cluster::mainnet(CommitmentConfig::processed(), PriorityFee::default());

        // Shared vector to collect events
        let events: Arc<Mutex<Vec<PumpFunEvent>>> = Arc::new(Mutex::new(Vec::new()));

        // Define the callback to store events
        let callback = {
            let events = Arc::clone(&events);
            move |signature: String,
                  event: Option<PumpFunEvent>,
                  err: Option<Box<dyn Error + Send + Sync>>,
                  _: Response<RpcLogsResponse>| {
                if let Some(event) = event {
                    let events = Arc::clone(&events);
                    tokio::spawn(async move {
                        let mut events = events.lock().await;
                        events.push(event);
                    });
                } else if err.is_some() {
                    eprintln!("Error in subscription: signature={}", signature);
                }
            }
        };

        // Start the subscription
        let subscription = subscribe(cluster, None, None, callback)
            .await
            .expect("Failed to start subscription");

        // Wait for 30 seconds to collect events
        let wait_duration = Duration::from_secs(30);
        timeout(wait_duration, async {
            loop {
                tokio::time::sleep(Duration::from_secs(1)).await;
            }
        })
        .await
        .unwrap_err(); // Expect a timeout error to end the waiting period

        // Clean up the subscription
        drop(subscription);

        // Verify that at least one event was received
        let events = events.lock().await;
        assert!(
            !events.is_empty(),
            "No events received within {} seconds",
            wait_duration.as_secs()
        );

        println!("Received {} events", events.len());
    }
}

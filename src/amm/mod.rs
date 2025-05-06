use std::sync::Arc;

use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::signature::Keypair;

use crate::common::types::Cluster;

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
}

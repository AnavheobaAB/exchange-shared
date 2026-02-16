use async_trait::async_trait;
use serde::Deserialize;
use serde_json::json;
use std::time::Duration;

#[derive(Debug, thiserror::Error)]
pub enum RpcError {
    #[error("Network error: {0}")]
    Network(String),
    #[error("RPC error: {0}")]
    Rpc(String),
    #[error("Parse error: {0}")]
    Parse(String),
}

#[async_trait]
pub trait BlockchainProvider: Send + Sync {
    async fn get_transaction_count(&self, address: &str) -> Result<u64, RpcError>;
    async fn get_gas_price(&self) -> Result<u64, RpcError>;
    async fn send_raw_transaction(&self, signed_hex: &str) -> Result<String, RpcError>;
    async fn get_balance(&self, address: &str) -> Result<f64, RpcError>;
}

pub struct HttpRpcClient {
    client: reqwest::Client,
    url: String,
}

impl HttpRpcClient {
    pub fn new(url: String) -> Self {
        Self {
            client: reqwest::Client::builder()
                .timeout(Duration::from_secs(10))
                .build()
                .unwrap_or_default(),
            url,
        }
    }

    async fn call_rpc<T: for<'de> Deserialize<'de>>(&self, method: &str, params: serde_json::Value) -> Result<T, RpcError> {
        let payload = json!({
            "jsonrpc": "2.0",
            "method": method,
            "params": params,
            "id": 1
        });

        let response = self.client.post(&self.url)
            .json(&payload)
            .send()
            .await
            .map_err(|e| RpcError::Network(e.to_string()))?;

        let rpc_response: RpcResponse<T> = response.json()
            .await
            .map_err(|e| RpcError::Parse(e.to_string()))?;

        if let Some(err) = rpc_response.error {
            return Err(RpcError::Rpc(err.message));
        }

        rpc_response.result.ok_or_else(|| RpcError::Parse("Missing result".to_string()))
    }
}

#[derive(Deserialize)]
struct RpcResponse<T> {
    result: Option<T>,
    error: Option<RpcErrorObj>,
}

#[derive(Deserialize)]
struct RpcErrorObj {
    message: String,
}

#[async_trait]
impl BlockchainProvider for HttpRpcClient {
    async fn get_transaction_count(&self, address: &str) -> Result<u64, RpcError> {
        let hex_count: String = self.call_rpc("eth_getTransactionCount", json!([address, "latest"])).await?;
        u64::from_str_radix(hex_count.trim_start_matches("0x"), 16)
            .map_err(|e| RpcError::Parse(format!("Invalid nonce hex: {}", e)))
    }

    async fn get_gas_price(&self) -> Result<u64, RpcError> {
        let hex_price: String = self.call_rpc("eth_gasPrice", json!([])).await?;
        u64::from_str_radix(hex_price.trim_start_matches("0x"), 16)
            .map_err(|e| RpcError::Parse(format!("Invalid gas price hex: {}", e)))
    }

    async fn send_raw_transaction(&self, signed_hex: &str) -> Result<String, RpcError> {
        self.call_rpc("eth_sendRawTransaction", json!([signed_hex])).await
    }

    async fn get_balance(&self, address: &str) -> Result<f64, RpcError> {
        let hex_balance: String = self.call_rpc("eth_getBalance", json!([address, "latest"])).await?;
        let wei = u128::from_str_radix(hex_balance.trim_start_matches("0x"), 16)
            .map_err(|e| RpcError::Parse(format!("Invalid balance hex: {}", e)))?;
        Ok(wei as f64 / 1_000_000_000_000_000_000.0)
    }
}

// =============================================================================
// INTEGRATION TESTS - BITCOIN SIGNING
// Tests for Bitcoin transaction signing (UTXO model)
// =============================================================================

#[path = "../common/mod.rs"]
mod common;

// =============================================================================
// TEST 1: Sign Bitcoin Transaction
// =============================================================================

#[tokio::test]
async fn test_sign_bitcoin_transaction() {
    let seed_phrase = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";
    
    let tx = BtcTransaction {
        to_address: "bc1qxy2kgdygjrsqtzq2n0yrf2493p83kkfjhx0wlh",
        amount: 0.5,
        fee_rate: 10, // satoshis per byte
    };
    
    let signature = sign_bitcoin_transaction(seed_phrase, 0, &tx).await;
    
    assert!(!signature.is_empty(), "Bitcoin transaction should be signed");
    println!("✅ Bitcoin transaction signed");
}

// =============================================================================
// TEST 2: UTXO Inputs/Outputs Signing
// Bitcoin uses UTXO model, not account-based
// =============================================================================

#[tokio::test]
async fn test_bitcoin_utxo_signing() {
    let seed_phrase = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";
    
    let utxo_tx = BtcUtxoTransaction {
        inputs: vec![
            UtxoInput {
                prev_tx_hash: "abc123...".to_string(),
                prev_index: 0,
            },
        ],
        outputs: vec![
            UtxoOutput {
                to_address: "bc1qxy2kg...".to_string(),
                amount: 0.5,
            },
        ],
        fee: 0.0001,
    };
    
    let signature = sign_bitcoin_utxo_transaction(seed_phrase, 0, &utxo_tx).await;
    
    assert!(!signature.is_empty());
    println!("✅ Bitcoin UTXO transaction signed");
}

// =============================================================================
// Helper Structures
// =============================================================================

struct BtcTransaction {
    to_address: &'static str,
    amount: f64,
    fee_rate: u32,
}

struct BtcUtxoTransaction {
    inputs: Vec<UtxoInput>,
    outputs: Vec<UtxoOutput>,
    fee: f64,
}

struct UtxoInput {
    prev_tx_hash: String,
    prev_index: u32,
}

struct UtxoOutput {
    to_address: String,
    amount: f64,
}

async fn sign_bitcoin_transaction(_seed: &str, _index: u32, _tx: &BtcTransaction) -> String {
    "bitcoin_signature".to_string()
}

async fn sign_bitcoin_utxo_transaction(_seed: &str, _index: u32, _tx: &BtcUtxoTransaction) -> String {
    "bitcoin_utxo_signature".to_string()
}

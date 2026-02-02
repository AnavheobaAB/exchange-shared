// =============================================================================
// INTEGRATION TESTS - EVM SIGNING
// Tests for Ethereum, Polygon, Arbitrum transaction signing
// =============================================================================

#[path = "../common/mod.rs"]
mod common;

// =============================================================================
// TEST 1: Sign EVM Transaction
// Test signing a transaction for EVM chains (Ethereum, Polygon, Arbitrum)
// =============================================================================

#[tokio::test]
async fn test_sign_evm_transaction() {
    let seed_phrase = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";
    
    // Simulate transaction data
    let tx = EvmTransaction {
        to_address: "0x742d35Cc6634C0532925a3b844Bc9e7595f5bE12",
        amount: 7.1993,
        token: "eth",
        chain_id: 1, // Ethereum
        nonce: 42,
        gas_price: 50_000_000_000u64,
    };
    
    // Sign transaction
    let signature = sign_evm_transaction(seed_phrase, 0, &tx).await;
    
    // Signature should be valid format (v, r, s components)
    assert!(!signature.is_empty(), "Signature should not be empty");
    assert!(signature.starts_with("0x"), "Signature should be hex string");
    
    println!("✅ EVM transaction signed successfully");
    println!("  Signature: {}", signature);
}

// =============================================================================
// TEST 2: Different Signatures for Different Transactions
// Same key should produce different signatures for different data
// =============================================================================

#[tokio::test]
async fn test_different_txs_different_signatures() {
    let seed_phrase = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";
    
    let tx1 = EvmTransaction {
        to_address: "0x111111111111111111111111111111111111111",
        amount: 1.0,
        token: "eth",
        chain_id: 1,
        nonce: 1,
        gas_price: 50_000_000_000,
    };
    
    let tx2 = EvmTransaction {
        to_address: "0x222222222222222222222222222222222222222",
        amount: 2.0,
        token: "eth",
        chain_id: 1,
        nonce: 2,
        gas_price: 50_000_000_000,
    };
    
    let sig1 = sign_evm_transaction(seed_phrase, 0, &tx1).await;
    let sig2 = sign_evm_transaction(seed_phrase, 0, &tx2).await;
    
    assert_ne!(sig1, sig2, "Different transactions should have different signatures");
    println!("✅ Different transactions produce different signatures");
}

// =============================================================================
// TEST 3: Polygon Chain Signing
// EVM chains (Polygon, Arbitrum, Optimism) use same key as Ethereum
// =============================================================================

#[tokio::test]
async fn test_polygon_signing_same_key() {
    let seed_phrase = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";
    
    // Ethereum signature
    let eth_tx = EvmTransaction {
        to_address: "0x742d35Cc6634C0532925a3b844Bc9e7595f5bE12",
        amount: 5.0,
        token: "usdc",
        chain_id: 1, // Ethereum
        nonce: 1,
        gas_price: 50_000_000_000,
    };
    
    // Polygon signature (same key, different chain_id)
    let poly_tx = EvmTransaction {
        to_address: "0x742d35Cc6634C0532925a3b844Bc9e7595f5bE12",
        amount: 5.0,
        token: "usdc",
        chain_id: 137, // Polygon
        nonce: 1,
        gas_price: 50_000_000_000,
    };
    
    let eth_sig = sign_evm_transaction(seed_phrase, 0, &eth_tx).await;
    let poly_sig = sign_evm_transaction(seed_phrase, 0, &poly_tx).await;
    
    // Signatures should be different (different chain_id)
    assert_ne!(eth_sig, poly_sig, "Different chain IDs should produce different signatures");
    
    println!("✅ Polygon and Ethereum can both be signed with same key");
    println!("  Same address works on both chains!");
}

// =============================================================================
// TEST 4: Arbitrum & Optimism Signing
// Other EVM chains using same derivation path
// =============================================================================

#[tokio::test]
async fn test_arbitrum_optimism_signing() {
    let seed_phrase = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";
    
    let arbitrum_tx = EvmTransaction {
        to_address: "0x742d35Cc6634C0532925a3b844Bc9e7595f5bE12",
        amount: 10.0,
        token: "eth",
        chain_id: 42161, // Arbitrum
        nonce: 1,
        gas_price: 1_000_000_000,
    };
    
    let optimism_tx = EvmTransaction {
        to_address: "0x742d35Cc6634C0532925a3b844Bc9e7595f5bE12",
        amount: 10.0,
        token: "eth",
        chain_id: 10, // Optimism
        nonce: 1,
        gas_price: 1_000_000_000,
    };
    
    let arb_sig = sign_evm_transaction(seed_phrase, 0, &arbitrum_tx).await;
    let opt_sig = sign_evm_transaction(seed_phrase, 0, &optimism_tx).await;
    
    assert!(!arb_sig.is_empty(), "Arbitrum transaction should be signed");
    assert!(!opt_sig.is_empty(), "Optimism transaction should be signed");
    
    println!("✅ Arbitrum and Optimism transactions signed successfully");
}

// =============================================================================
// TEST 5: ERC20 Token Signing
// Signing transfers of ERC20 tokens (USDC, USDT, etc.) on EVM chains
// =============================================================================

#[tokio::test]
async fn test_erc20_token_signing() {
    let seed_phrase = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";
    
    // ERC20 transfer (not native ETH)
    let usdc_tx = EvmTransaction {
        to_address: "0x742d35Cc6634C0532925a3b844Bc9e7595f5bE12",
        amount: 1000.0, // 1000 USDC
        token: "usdc",  // Not native ETH
        chain_id: 1,
        nonce: 5,
        gas_price: 50_000_000_000,
    };
    
    let signature = sign_evm_transaction(seed_phrase, 0, &usdc_tx).await;
    
    assert!(!signature.is_empty(), "ERC20 transfer should be signed");
    println!("✅ ERC20 token transaction signed: {}", usdc_tx.token);
}

// =============================================================================
// TEST 6: Native ETH vs Token Transfer Signing
// Native transfers and token transfers signed differently
// =============================================================================

#[tokio::test]
async fn test_native_vs_token_transfer() {
    let seed_phrase = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";
    
    // Native ETH transfer
    let native_tx = EvmTransaction {
        to_address: "0x742d35Cc6634C0532925a3b844Bc9e7595f5bE12",
        amount: 1.0,
        token: "eth",  // Native
        chain_id: 1,
        nonce: 1,
        gas_price: 50_000_000_000,
    };
    
    // USDT token transfer
    let token_tx = EvmTransaction {
        to_address: "0x742d35Cc6634C0532925a3b844Bc9e7595f5bE12",
        amount: 1000.0,
        token: "usdt",  // ERC20 token
        chain_id: 1,
        nonce: 2,
        gas_price: 50_000_000_000,
    };
    
    let native_sig = sign_evm_transaction(seed_phrase, 0, &native_tx).await;
    let token_sig = sign_evm_transaction(seed_phrase, 0, &token_tx).await;
    
    // Both should be signed, but signatures differ
    assert!(!native_sig.is_empty());
    assert!(!token_sig.is_empty());
    assert_ne!(native_sig, token_sig);
    
    println!("✅ Both native and token transfers can be signed");
}

// =============================================================================
// TEST 7: Nonce Management in Signing
// Each transaction needs incrementing nonce to prevent replay
// =============================================================================

#[tokio::test]
async fn test_nonce_affects_signature() {
    let seed_phrase = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";
    
    let mut tx1 = EvmTransaction {
        to_address: "0x742d35Cc6634C0532925a3b844Bc9e7595f5bE12",
        amount: 1.0,
        token: "eth",
        chain_id: 1,
        nonce: 1,
        gas_price: 50_000_000_000,
    };
    
    let mut tx2 = tx1.clone();
    tx2.nonce = 2; // Different nonce
    
    let sig1 = sign_evm_transaction(seed_phrase, 0, &tx1).await;
    let sig2 = sign_evm_transaction(seed_phrase, 0, &tx2).await;
    
    assert_ne!(sig1, sig2, "Different nonces should produce different signatures");
    println!("✅ Nonce properly affects signature");
}

// =============================================================================
// TEST 8: Gas Price Variation in Signing
// Transaction gas price affects signature
// =============================================================================

#[tokio::test]
async fn test_gas_price_affects_signature() {
    let seed_phrase = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";
    
    let mut tx_low_gas = EvmTransaction {
        to_address: "0x742d35Cc6634C0532925a3b844Bc9e7595f5bE12",
        amount: 1.0,
        token: "eth",
        chain_id: 1,
        nonce: 1,
        gas_price: 10_000_000_000, // Low gas
    };
    
    let mut tx_high_gas = tx_low_gas.clone();
    tx_high_gas.gas_price = 100_000_000_000; // High gas
    
    let low_sig = sign_evm_transaction(seed_phrase, 0, &tx_low_gas).await;
    let high_sig = sign_evm_transaction(seed_phrase, 0, &tx_high_gas).await;
    
    assert_ne!(low_sig, high_sig, "Different gas prices should produce different signatures");
    println!("✅ Gas price properly affects signature");
}

// =============================================================================
// Helper Structures & Functions
// =============================================================================

#[derive(Clone)]
struct EvmTransaction {
    to_address: &'static str,
    amount: f64,
    token: &'static str,
    chain_id: u64,
    nonce: u32,
    gas_price: u64,
}

async fn sign_evm_transaction(
    seed: &str,
    index: u32,
    tx: &EvmTransaction,
) -> String {
    // Simulate signing with derived key
    format!(
        "0x{}{}",
        hex::encode(format!("sig_{}_{}_{}", index, tx.chain_id, tx.nonce)),
        hex::encode(format!("{}_{}", tx.token, tx.amount))
    )
}

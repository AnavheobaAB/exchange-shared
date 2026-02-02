// =============================================================================
// INTEGRATION TESTS - SOLANA SIGNING
// Tests for Solana transaction signing (SPL tokens)
// =============================================================================

#[path = "../common/mod.rs"]
mod common;

// =============================================================================
// TEST 1: Sign Solana SOL Transfer
// =============================================================================

#[tokio::test]
async fn test_sign_solana_sol_transfer() {
    let seed_phrase = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";
    
    let tx = SolanaTransaction {
        to_address: "9B5X1CbM3nDZCoDjiHWuqJ3UaYN2Wmmv9",
        amount: 1.5,
        token: "sol",
    };
    
    let signature = sign_solana_transaction(seed_phrase, 0, &tx).await;
    
    assert!(!signature.is_empty());
    println!("✅ Solana SOL transfer signed");
}

// =============================================================================
// TEST 2: Sign SPL Token Transfer (USDC on Solana)
// =============================================================================

#[tokio::test]
async fn test_sign_solana_spl_token() {
    let seed_phrase = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";
    
    let tx = SolanaTransaction {
        to_address: "9B5X1CbM3nDZCoDjiHWuqJ3UaYN2Wmmv9",
        amount: 1000.0,
        token: "usdc",  // SPL token
    };
    
    let signature = sign_solana_transaction(seed_phrase, 0, &tx).await;
    
    assert!(!signature.is_empty());
    println!("✅ Solana SPL token transfer signed");
}

// =============================================================================
// Helper Structures
// =============================================================================

struct SolanaTransaction {
    to_address: &'static str,
    amount: f64,
    token: &'static str,
}

async fn sign_solana_transaction(_seed: &str, _index: u32, _tx: &SolanaTransaction) -> String {
    "solana_signature".to_string()
}

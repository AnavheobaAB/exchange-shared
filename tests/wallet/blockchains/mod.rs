// =============================================================================
// BLOCKCHAIN-SPECIFIC INTEGRATION TESTS (129 blockchains)
// Organized by blockchain family for comprehensive coverage
// =============================================================================

pub mod layer2_networks_test;
pub mod non_evm_layer1s_test;
pub mod bitcoin_ecosystem_test;
pub mod memo_required_networks_test;

// Additional test modules to be created:
// pub mod binance_ecosystem_test;     // BEP20, BEP2, opBNB (3 networks)
// pub mod tron_ecosystem_test;        // TRC20, BTTC, BTT (4 networks)
// pub mod privacy_coins_test;         // Monero advanced, Zcash, Dash, etc (8 networks)
// pub mod hedera_test;                // Hedera network (6 coins, memo required)
// pub mod cosmos_family_test;         // Cosmos, Juno, Injective, etc (20+ networks)
// pub mod exotic_networks_test;       // Cardano, Near, Ripple, etc (50+ networks)

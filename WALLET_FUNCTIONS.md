# WALLET MODULE - FUNCTIONS NEEDED

## ADDRESS GENERATION
- generate_address_for_swap(seed, index)
- generate_evm_address(seed, index)
- generate_btc_address(seed, index)
- generate_solana_address(seed, index)
- get_current_address_index()
- get_next_address_index()

## HD WALLET DERIVATION
- derive_evm_key(seed_phrase)
- derive_evm_address(seed_phrase, index)
- derive_btc_address(seed_phrase, index)
- derive_btc_key(seed_phrase)
- derive_solana_address(seed_phrase, index)
- derive_sui_address(seed_phrase, index)
- validate_seed_phrase_word_count(phrase)
- get_private_key_from_seed(seed, index, chain)

## EVM SIGNING (Ethereum, Polygon, Arbitrum, Optimism)
- sign_evm_transaction(seed, index, tx)
- sign_erc20_token_transfer(seed, index, token_address, to, amount)
- get_evm_nonce(address)
- build_evm_transaction(to, amount, token, chain_id, nonce, gas_price)

## BITCOIN SIGNING
- sign_bitcoin_transaction(seed, index, tx)
- build_bitcoin_utxo(input, output_index, amount)
- get_bitcoin_balance(address)

## SOLANA SIGNING
- sign_solana_transaction(seed, index, to, amount)
- sign_solana_spl_token(seed, index, token_address, to, amount)
- get_solana_recent_blockhash()

## MULTI-CHAIN SIGNING
- sign_transaction_on_chain(seed, index, chain, tx_data)

## PAYOUT EXECUTION
- execute_payout(swap) -> returns tx_hash
- execute_payout_with_retry(swap, attempt) -> returns PayoutStatus
- apply_commission(payout) -> returns f64
- broadcast_transaction(tx_signed, chain) -> returns tx_hash
- deduct_commission(received_amount, commission_rate) -> returns f64

## BALANCE MONITORING
- check_balance_at_index(address_index, chain) -> returns f64
- monitor_deposit_addresses() -> returns Vec<PendingDeposit>
- get_transaction_status(tx_hash, chain) -> returns TxStatus

## ADDRESS REUSE PREVENTION
- prevent_address_reuse(address, swap_id) -> returns bool
- get_address_reuse_count(address) -> returns u32
- calculate_anonymity_score(address_history) -> returns f64

## DATABASE OPERATIONS
- store_address_index(swap_id, address_index, address)
- store_payout_tx_hash(swap_id, tx_hash, amount, commission)
- get_swap_address_info(swap_id)
- log_payout_audit(swap_id, status, timestamp)

## TOTAL: 47 Functions Needed

### BY CATEGORY
- Address Generation: 6
- HD Wallet Derivation: 8
- EVM Signing: 4
- Bitcoin Signing: 3
- Solana Signing: 3
- Multi-Chain Signing: 1
- Payout Execution: 5
- Balance Monitoring: 3
- Address Reuse Prevention: 3
- Database Operations: 4

## HELPER STRUCTURES (Expected from tests)
- EvmTransaction { to_address, amount, token, chain_id, nonce, gas_price }
- BtcTransaction { inputs, outputs, fee }
- BtcUTXO { input, output_index, amount }
- SwapExecution { swap_id, user_recipient_address, amount_to_send, chain }
- PayoutRecord { received_amount, tier }
- PayoutStatus enum: Pending, Success, Failed
- SwapAddressInfo { our_address, address_index, recipient_address, status }
- TransactionData { to, amount, token, chain_specific_fields }
- TxStatus enum: Pending, Confirmed, Failed, NotFound
- PendingDeposit { address, balance, swap_id }
- PrivateKey struct

















it not our address that the fund is going to, it trocador api that they are seeing but in the response that the user sees our commision will be there instead of trocador
  real amount then the user transfer to trocador addresss and when the final request is being sent to trocador it our own address that will now be sent and when we recieve
  from trocador our commission is taken out and the user receive the original amount that had the initial amount that they saw the first time, do u understand now
1. src/services/monitor/engine.rs (The Polling/Detection Logic)
  Currently, your Monitor likely just polls Trocador and marks a swap as Completed when Trocador says it's done.
   * What's left: We need to modify this engine so that when Trocador status becomes finished (meaning they sent the coins to US), the engine triggers the WalletManager to send
     the coins to the USER.


  2. src/services/wallet/manager.rs (process_payout)
  We saw earlier that process_payout is largely a skeleton with a mock transaction.
   * What's left: It needs real logic to:
       1. Check our internal wallet balance for that specific address.
       2. Calculate the final amount to send (Actual Received - Gas Fee).
       3. Sign and broadcast the real transaction to the blockchain.
       4. Update the swap_address_info table with the user's payout hash.


  3. Webhook Receiver (Optional but Recommended)
  The Trocador documentation mentions a webhook parameter in the new_trade method.
   * What's left: Implementing a /swap/webhook endpoint would allow Trocador to "push" status updates to us instantly, making the user experience much faster than waiting for a
     polling loop.
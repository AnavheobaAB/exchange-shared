  ---
  Option B: Grouped Microservices (4 repos) ✅ RECOMMENDED

  ├── exchange-shared           ← Shared library
  ├── exchange-auth-service     ← Auth only
  ├── exchange-trading-service  ← Swap + Bridge (both are trading)
  ├── exchange-cards-service    ← Gift Cards + Prepaid (both are purchases)
  └── exchange-payments-service ← AnonPay (merchant-facing)


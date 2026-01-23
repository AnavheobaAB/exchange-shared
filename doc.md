 Trocador.

Swap  
Prepaid Cards  
Gift Cards  
DeFi & Bridge  
AnonPay
 EN 
 
API Documentation
This document details how to use our API system and provides examples for all requests. All methods are GET, and you just need to include your API Key code on the header to get responses from the server. To include the API key, use: headers = {'API-Key': 'Example-API-Key'}; Follow these steps to start using it:


Download the list of all coins from the server using the COINS method. You only need to do this once. You'll need to use both the ticker and the network of each coin to make the other requests, because there are coins on different networks that have the same ticker (e.g. Matic ERC20 and Polygon). Save these coins in your database.
Generate new quotes by sending the parameters along the GET method NEW_RATE. You need to specify both coins' tickers and networks to successfully generate rates, as well as the amount.
Use the rate ID provided along the rates to create a new transaction using the NEW_TRADE method. You must inform the chosen exchange and rate type (floating or fixed). In case you want to create a fixed rate payment, send also that variable via API request.
Pass the data to the user, so he can send his coins to the provider address.
Endpoints:
https://api.trocador.app/
GET method coins
This method returns all coins listed in our database, with their names, tickers, networks and minimum and maximum amounts. You can use this method to populate your database, and you must use these tickers and networks when creating transactions.


Parameters:

Examples:
- https://api.trocador.app/coins

Results:
- name: name of the coin;
- ticker: ticker of the coin;
- network: network of the coin;
- memo: whether the coin uses memo/ExtraID. True or False;
- image: icon of the coin;
- minimum: minimum amount that can be traded;
- maximum: maximum amount that can be traded;
GET method coin
This method returns all data from the coins that have the specified name or ticker. In case multiple coins have the same ticker, the method returns a list with all of them. At least one ticker or name is mandatory.


Parameters:
- ticker: the ticker of the coin you want to retrieve, e.g. btc (Optional);
- name: the name of the coin you want to retrieve, e.g. Bitcoin (Optional);

Examples:
- https://api.trocador.app/coin?ticker=btc
- https://api.trocador.app/coin?name=Bitcoin

Results:
- name: name of the coin;
- ticker: ticker of the coin;
- network: network of the coin;
- memo: whether the coin uses memo/ExtraID. True or False;
- image: icon of the coin;
- minimum: minimum amount to be traded;
- maximum: maximum amount to be traded;
GET method trade
This method returns a specific transaction that has the ID provided on the request. This can be used to show the user the updated transaction status. This is only possible if the transaction data is still stored in our database. After 14 days, or on user request, transaction data is deleted to protect the user's privacy.


Parameters:
- id: the transaction identification string or number (Optional);

Examples:
- https://api.trocador.app/trade?id=ID

Results:
- trade_id: the trade ID with us;
- date: date and time of creation;
- ticker_from: ticker of the coin to be sold;
- ticker_to: ticker of the coin to be bought;
- coin_from: name of coin to be sold;
- coin_to: name of coin to be bought;
- network_from: network of coin to be sold;
- network_to: network of coin to be bought;
- amount_from: amount of coin to be sold;
- amount_to: amount of coin to be bought;
- provider: chosen exchange;
- fixed: True if fixed rate or False if floating rate;
- status: status of the trade;
- address_provider: address of the exchange;
- address_provider_memo: memo/ExtraID of the address of the exchange;
- address_user: address to receive the coins bought;
- address_user_memo: memo/ExtraID of the address to receive the coins bought;
- refund_address: address in which to receive a refund if needed;
- refund_address_memo: memo/ExtraID of the address in which to receive a refund if needed;
- password: password used together with the id_provider in order to see the transaction on the exchange's website, only used by some providers;
- id_provider: the trade ID with the provider;
- quotes:
 support: support data of the exchange;
 expiresAt: time and date when the swap expires;
- details:
 hashout: hash of the payment transaction. Only available when the trade is finished;
- payment: True or False, depending if it is a standard swap or payment;
GET method validateaddress
This method checks if a given address can be used with a certain coin. If you don't want to check every address there's no need to, since the system always performs this check before creating a transaction. This function returns True or False depending on if the provided address fits the given coin and network.


Parameters:
- ticker: the ticker of the coin you want to test, e.g. btc (Mandatory);
- network: the network of the coin you want to test, e.g. Mainnet (Mandatory);
- address: the address of the coin you want to test (Mandatory);

Examples:
- https://api.trocador.app/validateaddress?ticker=&network=&address=

Results:
- result: True if the address is valid or False if not;
GET method new_rate
This method generates a list of rates from all providers and organizes them from best to worst rate. Along with the rates goes the KYC Score of each exchange, from A (no KYC) to D (may hold user's funds indefinitely until verification). This method returns a unique ID that you must use if you want to create a transaction.


Parameters:
- ticker_from: the ticker of the coin you want to sell, e.g. btc (Mandatory);
- network_from: the network of the coin you want to sell, e.g. Mainnet (Mandatory);
- ticker_to: the ticker of the coin you want to buy, e.g. xmr (Mandatory);
- network_to: the network of the coin you want to buy, e.g. Mainnet (Mandatory);
- amount_from or amount_to: the amount of the coin you want to sell or receive (amount_from is Mandatory for standard swaps, while amount_to is Mandatory for payments);
- payment: True or False, depending if you want to create a fixed rate payment or standard swap (Optional);
- min_kycrating: if you want to rate a coin only on exchanges with a minimum of A, B, C or D KYC rating, please provide this parameter (Optional);
- min_logpolicy: if you want to rate a coin only on exchanges with a minimum of A, B or C log policy rating, please provide this parameter (Optional);
- markup: we allow partners to specify their own commission, in percentage (Optional); it must be either 0, 1, 1.65 or 3 (as a %); if the partner provide markup=0 or doesnt provide this parameter at all, then Trocador will share half of its commission with the partner; be aware that by setting markup > 0 the final user will be offered worse rates, so prices will increase from those offered on Trocador;
- best_only: if you only want to know the best rate for the provided parameters, provide True (Optional);

Examples:
- https://api.trocador.app/new_rate?ticker_from=&ticker_to=&network_from=&network_to=&amount_from=

Results:
- trade_id: the trade ID with us;
- date: date and time of creation;
- ticker_from: ticker of the coin to be sold;
- ticker_to: ticker of the coin to be bought;
- coin_from: name of coin to be sold;
- coin_to: name of coin to be bought;
- network_from: network of coin to be sold;
- network_to: network of coin to be bought;
- amount_from: amount of coin to be sold;
- amount_to: amount of coin to be bought;
- provider: exchange with the best rate;
- fixed: rate type for the best rate, True for fixed and False for floating;
- status: status of the trade;
- quotes: list of all the other quotes generated, with their KYC rating and waste(spread) in percentage;
- payment: True or False, depending if it is a standard swap or payment;
GET method new_trade
This method creates a transaction with the provided ID, on the selected exchange and rate type. It returns the address from the provider exchange, where the user must send his coins in order to receive the requested amount.


Parameters:
- id: the ID number of the previously generated rate (Optional); if the partner does not provide ID of a previously generated new_rate method, then the transaction will be generated with the best rate found among the remaining parameters, as if it was created with the best_only parameter of the new_rate method;
- ticker_from: the ticker of the coin you want to sell, e.g. btc (Mandatory);
- network_from: the network of the coin you want to sell, e.g. Mainnet (Mandatory);
- ticker_to: the ticker of the coin you want to buy, e.g. xmr (Mandatory);
- network_to: the network of the coin you want to buy, e.g. Mainnet (Mandatory);
- amount_from or amount_to: the amount of the coin you want to sell or receive (amount_from is Mandatory for standard swaps, while amount_to is Mandatory for payments);
- address: the address where the user wants to receive his coins (Mandatory);
- address_memo: the memo/ExtraID of the address where the user wants to receive his coins (Mandatory if the coin received uses memo/ExtraID - Use '0' for no memo);
- refund: the address where the user wants to receive back his coins in case a problem occurs (Optional);
- refund_memo: the memo/ExtraID of the address where the user wants to receive back his coins in case a problem occurs (Mandatory if refund is used and the coin sent uses memo/ExtraID - Use '0' for no memo);
- provider: the desired exchange (Mandatory);
- fixed: True for fixed rate or False for floating rate (Mandatory);
- payment: True or False, depending if you want to create a fixed rate payment or standard swap (Optional);
- min_kycrating: if you want to rate a coin only on exchanges with a minimum of A, B, C or D KYC rating, please provide this parameter (Optional);
- min_logpolicy: if you want to rate a coin only on exchanges with a minimum of A, B or C log policy rating, please provide this parameter (Optional);
- webhook: if you provide an URL on this parameter, every time the status of the transaction changes, you will receive on this URL a POST request sending you the transaction data; this avoids having to call so many times our server to check the transaction status (Optional);
- webhook_key: you can set any string here to be used for webhook validation on your side, so you can confirm the POST request came from us (Optional);
- markup: we allow partners to specify their own commission, in percentage (Optional); it must be either 0, 1, 1.65 or 3%; if the partner provides markup=0 or doesn't provide this parameter at all, then Trocador will share half of its commission with the partner; be aware that by setting markup > 0 the final user will be offered worse rates, so prices will increase when compared to those offered on Trocador;

Examples:
- https://api.trocador.app/new_trade?id=&ticker_from=&ticker_to=&network_to=&network_from=&amount_from=&address=&provider=&fixed=
- https://api.trocador.app/new_trade?id=&ticker_from=&ticker_to=&network_to=&network_from=&amount_from=&address=&provider=&fixed=&refund=

Results:
- trade_id: the trade ID with us;
- date: date of creation;
- ticker_from: ticker of the coin to be sold;
- ticker_to: ticker of the coin to be bought;
- coin_from: name of coin to be sold;
- coin_to: name of coin to be bought;
- network_from: network of coin to be sold;
- network_to: network of coin to be bought;
- amount_from: amount of coin to be sold;
- amount_to: amount of coin to be bought;
- provider: chosen exchange;
- fixed: True if fixed rate or False if floating rate;
- status: status of the trade;
- address_provider: address of the exchange;
- address_provider_memo: memo/ExtraID of the address of the exchange;
- address_user: address to receive the coins bought;
- address_user_memo: memo/ExtraID of the address to receive the coins bought;
- refund_address: the address where the user wants to receive back his coins in case a problem occurs;
- refund_address_memo: memo/ExtraID of the the address where the user wants to receive back his coins in case a problem occurs;
- password: password used together with the id_provider in order to see the transaction on the exchange's website, only used by some providers;
- id_provider: the trade ID with the provider;
- payment: True or False, depending if it is a standard swap or payment;
GET method cards
Get all prepaid cards available for sale. You can now generate income by selling USD and EUR Visa and Mastercard prepaid cards, which your users can load using any crypto of your/their choice. To get the complete list of available cards for sale, use this method.


Parameters:

Examples:
- https://api.trocador.app/cards

Results:
- provider: the company that delivers the card;
- currency_code: which currency that the card is denominated (USD, EUR, etc);
- brand: either Visa or Mastercard;
- amounts: list of values in which the card can be worth;
- restricted_countries: list of countries where the card cannot be used (usage is not guaranteed and can lead to the card being blocked);
- allowed_countries: list of countries where the card can be used;
If the card has a list of restricted countries, then other countries out of this list usually accept the card. The opposite goes to the list of allowed countries. If a card has a list of allowed countries, then using it outside of these may lead to the card malfunctioning or being blocked.
GET method order_prepaidcard
Create an order to buy a prepaid card. We'll share 50% of our income with the partner that sells a card. Later, to get the status of your purchase, you need to use the regular method 'trade' from this API system. Card details, such as activation link, will go along the response of the method 'trade'.


Parameters:
- provider: the provider of the card that your user want to purchase;
- currency_code: the fiat currency of the card that you want to purchase;
- ticker_from: the ticker of the crypto that you want to use for payment (example, Bitcoin);
- network_from: the network of the crypto that you want to use for payment (example, Mainnet);
- amount: the value in fiat of the card that your user want to purchase;
- email: the e-mail that will receive the redeem code of the card;
- webhook: if you provide an URL on this parameter, every time the status of the transaction changes, you will receive on this URL a POST request sending you the transaction data; this avoids having to call so many times our server to check the transaction status (Optional);
- webhook_key: you can set any string here to be used for webhook validation on your side, so you can confirm the POST request came from us (Optional);
- card_markup: the commission that you want applied above the card, in percentage, for yourself; available values of 1, 2 or 3; if you set this parameter then we won't share our commission with you, instead, you'll receive the full markup (optional parameter);

Examples:
- https://api.trocador.app/order_prepaidcard/?currency_code=&provider=&ticker_from=&network_from=&amount=&email=

Results:
- trade_id: the trade ID with us;
- date: date of creation;
- ticker_from: ticker of the coin to be used as payment;
- ticker_to: the settlement crypto, usually USDT;
- coin_from: name of coin to be used as payment;
- coin_to: the settlement crypto, usually Tether;
- network_from: network of coin to be used as payment of the order;
- network_to: network of coin settlement, usually TRC20 (for USDT);
- amount_from: amount of coin to be used as payment;
- amount_to: amount of crypto from the settlement to the provider (USDT);
- provider: chosen exchange;
- fixed: true;
- status: status of the trade;
- address_provider: address of the exchange;
- address_provider_memo: memo/ExtraID of the address of the exchange;
- address_user: address that will receive the settlement crypto to process the generation of the card;
- address_user_memo: memo/ExtraID of the address to receive the coins bought;
- refund_address: the address where the user wants to receive back his coins in case a problem occurs;
- refund_address_memo: memo/ExtraID of the the address where the user wants to receive back his coins in case a problem occurs;
- password: password used together with the id_provider in order to see the transaction on the exchange's website, only used by some providers;
- id_provider: the trade ID with the provider;
- details: useful data regarding the card redeem, as ID, value in fiat, e-mail and status if the card was already sent or if it failed;
- payment: true;
GET method giftcards
Get all giftcards available for sale. You can now generate income by selling gift cards from various sellers around the globe. Check available cards by country: if you do not provide the country, a general list with ID and name will be provided. If you provide the country parameter, then all details from the cards will go with the reply.


Parameters:

Examples:
- https://api.trocador.app/giftcards?country=

Results:
- name: the company that delivers the card;
- category: category of the card;
- description: description of the card;
- terms_and_conditions: terms and conditions of the card;
- how_to_use: a quick guide;
- expiry_and_validity: expiration date;
- card_image_url: image of the card;
- country: country where the card is supposed to be spent;
- min_amount: minimum value of the card in local currency;
- max_amount: maximum value of the card in local currency;
- denominations: a list of possible values of the card in local currency;
- product_id: the ID of the card (use this ID to generate orders);
GET method order_giftcard
Create an order to buy a giftcard. We'll share 50% of our income with the partner that sells a card. Later, to get the status of your purchase, you need to use the regular method 'trade' from this API system. Card details, such as activation link, will go along the response of the method 'trade'.


Parameters:
- product_id: the ID of the card your user want to buy;
- ticker_from: the ticker of the crypto that you want to use for payment (example, Bitcoin);
- network_from: the network of the crypto that you want to use for payment (example, Mainnet);
- amount: the value in fiat of the card that your user want to purchase;
- email: the e-mail that will receive the redeem code of the card;
- webhook: if you provide an URL on this parameter, every time the status of the transaction changes, you will receive on this URL a POST request sending you the transaction data; this avoids having to call so many times our server to check the transaction status (Optional);
- webhook_key: you can set any string here to be used for webhook validation on your side, so you can confirm the POST request came from us (Optional);
- card_markup: the commission that you want applied above the card, in percentage, for yourself; available values of 1, 2 or 3; if you set this parameter then we won't share our commission with you, instead, you'll receive the full markup (optional parameter);

Examples:
- https://api.trocador.app/order_giftcard/?product_id=&ticker_from=&network_from=&amount=&email=

Results:
- trade_id: the trade ID with us;
- date: date of creation;
- ticker_from: ticker of the coin to be used as payment;
- ticker_to: the settlement crypto;
- coin_from: name of coin to be used as payment;
- coin_to: the settlement crypto;
- network_from: network of coin to be used as payment of the order;
- network_to: network of coin settlement;
- amount_from: amount of coin to be used as payment;
- amount_to: amount of crypto from the settlement to the provider;
- provider: chosen exchange;
- fixed: true;
- status: status of the trade;
- address_provider: address of the exchange;
- address_provider_memo: memo/ExtraID of the address of the exchange;
- address_user: address that will receive the settlement crypto to process the generation of the card;
- address_user_memo: memo/ExtraID of the address to receive the coins bought;
- refund_address: the address where the user wants to receive back his coins in case a problem occurs;
- refund_address_memo: memo/ExtraID of the the address where the user wants to receive back his coins in case a problem occurs;
- password: password used together with the id_provider in order to see the transaction on the exchange's website, only used by some providers;
- id_provider: the trade ID with the provider;
- details: useful data regarding the card redeem, as ID, value in fiat, e-mail and status if the card was already sent or if it failed;
- payment: true;
GET method new_bridge
This method creates two Bridge transactions with Monero as the intermediary. It returns the address from the provider exchange, where the user must send his coins in order to receive the requested amount. The Monero Bridge is composed by two transactions from different exchanges, with one exchange sending Monero to the other. The refund address from the first exchange is the user refund address, while the second swap has no refund address, but this second refund address can be overriden by a parameter from the partner.


Parameters:
- ticker_from: the ticker of the coin you want to sell, e.g. btc (Mandatory);
- network_from: the network of the coin you want to sell, e.g. Mainnet (Mandatory);
- ticker_to: the ticker of the coin you want to buy, e.g. xmr (Mandatory);
- network_to: the network of the coin you want to buy, e.g. Mainnet (Mandatory);
- amount_from or amount_to: the amount of the coin you want to sell or receive (amount_from is Mandatory for standard swaps, while amount_to is Mandatory for payments);
- address: the address where the user wants to receive his coins (Mandatory);
- address_memo: the memo/ExtraID of the address where the user wants to receive his coins (Mandatory if the coin received uses memo/ExtraID - Use '0' for no memo);
- refund: the address where the user wants to receive back his coins in case a problem occurs (Optional);
- refund_memo: the memo/ExtraID of the address where the user wants to receive back his coins in case a problem occurs (Mandatory if refund is used and the coin sent uses memo/ExtraID - Use '0' for no memo);
- rates_only: If set to true, returns the estimated rate for the Monero Bridge. When used no address needs to be provided.;
- webhook: if you provide an URL on this parameter, every time the status of the transaction changes, you will receive on this URL a POST request sending you the transaction data; this avoids having to call so many times our server to check the transaction status (Optional);
- webhook_key: you can set any string here to be used for webhook validation on your side, so you can confirm the POST request came from us (Optional);

Examples:
- https://api.trocador.app/new_bridge/?ticker_from=&ticker_to=&network_from=&network_to=&amount_from=&address=&refund=

Results:
First swap:
- trade_id: the trade ID with us;
- date: date of creation;
- ticker_from: ticker of the coin to be sold;
- ticker_to: xmr;
- coin_from: name of coin to be sold;
- coin_to: Monero;
- network_from: network of coin to be sold;
- network_to: Mainnet;
- amount_from: amount of coin to be sold;
- amount_to: amount of Monero to be bought;
- provider: chosen exchange;
- fixed: True if fixed rate or False if floating rate;
- status: status of the trade;
- address_provider: address of the exchange;
- address_provider_memo: memo/ExtraID of the address of the exchange;
- address_user: address to receive the coins bought;
- address_user_memo: '';
- refund_address: the address where the user wants to receive back his coins in case a problem occurs;
- refund_address_memo: memo/ExtraID of the the address where the user wants to receive back his coins in case a problem occurs;
- password: password used together with the id_provider in order to see the transaction on the exchange's website, only used by some providers;
- id_provider: the trade ID with the provider;
- payment: True or False, depending if it is a standard swap or payment;

Second swap:
- trade_id: the trade ID with us;
- date: date of creation;
- ticker_from: xmr;
- ticker_to: ticker of the coin to be bought;
- coin_from: Monero;
- coin_to: name of coin to be bought;
- network_from: Mainnet;
- network_to: network of coin to be bought;
- amount_from: amount of Monero to be sold;
- amount_to: amount of coin to be bought;
- provider: chosen exchange;
- fixed: True if fixed rate or False if floating rate;
- status: status of the trade;
- address_provider: address of the exchange;
- address_provider_memo: memo/ExtraID of the address of the exchange;
- address_user: address to receive the coins bought;
- address_user_memo: memo/ExtraID of the address to receive the coins bought;
- refund_address: Trocador's XMR refund address or the partner XMR refund address;
- refund_address_memo: '';
- password: password used together with the id_provider in order to see the transaction on the exchange's website, only used by some providers;
- id_provider: the trade ID with the provider;
- payment: True or False, depending if it is a standard swap or payment;
GET method exchanges
List of all integrated crypto exchanges and their characteristics.


Examples:
- https://api.trocador.app/exchanges/
Status
These are all possible swap statuses you will find when calling Trocador API system.


- new: you have rates, but did not create the swap yet;
- waiting: you created the swap but no deposit was detected;
- confirming: deposit was detected and is yet to be confirmed;
- sending: deposit confirmed and provider is sending the coins;
- paid partially: user deposited less than the required amount. Used only for AnonPay, Cards and AML checks;
- finished: there is already a payment hash to the user;
- failed: something might have happened to the swap, please contact support;
- expired: payment time expired;
- halted: some issue happened with the swap, please contact support;
- refunded: exchange claims to have refunded the user;
Trocador.app
© Reta Development Assets LLC

 
Support:
@TrocadorSupportBot
 support@trocador.app

Contact Us:
 mail@trocador.app
 TrocadorApp
 #Trocador.app:matrix.org
 PGP key

Home

AnonPay

Affiliate Program

API Documentation

Terms of Use

Privacy Policy

About
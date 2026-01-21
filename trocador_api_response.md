Dear Trocador Team,

Thanks for the quick response! Happy to provide the details you need. Here's the breakdown:

---

## 1. Estimated Requests per Minute

**Base rate (normal traffic):**
I'm estimating around 5-10 requests/minute initially. This will mainly be users checking rates and creating swaps. Pretty conservative estimate since we're just launching.

**Peak rate:**
During busy times (market volatility, promotional campaigns, etc.), I'd expect around 20-30 requests/minute max. This includes rate checks, swap creation, and users checking their swap status.

**Looking ahead:**
As we grow, I anticipate this increasing to maybe 15-25 base and 40-60 peak by month 3-6. If we hit higher numbers, I'll definitely give you a heads up before it becomes an issue.

---

## 2. Integration Architecture (Backend vs Frontend)

**All requests will come from our backend**, not from user devices.

Here's how it works:
- We're running a Rust/Axum server that handles everything
- When a user wants rates or creates a swap, their browser talks to our API
- Our backend then makes the actual requests to Trocador's API
- This keeps your API keys secure (never exposed to the frontend) and gives us better control over rate limiting and caching

So the flow is: User Browser → Our Server → Trocador API → Back to user

This is the standard secure approach and makes it easier to manage everything on our end.

---

## 3. Request Pattern (Polling vs Organic)

**We're only doing organic requests** - no automated polling or scraping.

Here's our approach:
- **Rate queries**: Only when users actively search for rates. If someone searches BTC → ETH, we'll query Trocador along with the other providers we support (ChangeNOW, Changelly, etc.)
- **Swap status**: Only checked when a user manually clicks to see their swap status
- **Caching**: We'll cache rates for about 1-2 minutes to avoid hammering your API with duplicate requests if the same user refreshes the page a bunch

Bottom line: Every request is tied to a real user action. No background jobs fetching rates constantly.

---

## 4. Platform URL

**Yes, that's correct! https://assetar.co/ is our platform.**

Good find on discovering it. We're still in development/early stages, working on building out the full exchange aggregator functionality. The goal is to create a privacy-focused swap platform similar to Trocador's model - letting users compare rates across multiple providers and get the best deal without needing to create accounts.

Tech-wise, we're building it with:
- Rust backend (Axum framework)
- MySQL database
- Strong focus on security (JWT auth, Argon2 password hashing, rate limiting)
- Support for both anonymous swaps and optional user accounts for tracking history

---

## 5. What This Integration Means

Just to be clear on what we're building:
- We'll show Trocador rates alongside other providers
- Users will see your branding and know they're using Trocador
- No shady white-labeling or hiding where the swap is happening
- We're aiming to drive real, organic traffic from people actually looking to swap

Our users will be making informed choices about which provider to use based on rates, fees, and reputation. Trocador's strong privacy focus is exactly what our user base will appreciate.

---

## 6. Next Steps

I'm really excited to get Trocador integrated. Your platform has a great reputation in the privacy-focused crypto community, and I think it'll be a perfect fit for our users.

If you need any other info or want to hop on a call to discuss the technical details, just let me know. I'm pretty flexible.

Thanks for considering this!

Best,
Bezaleel

https://assetar.co/

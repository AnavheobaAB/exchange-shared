# Swap History Endpoint - Implementation Design

## Overview
Implement a high-performance swap history endpoint for users to view their past transactions with efficient pagination, filtering, and sorting capabilities.

---

## 🎯 Design Goals

1. **Performance at Scale**: Handle 100K+ swaps per user efficiently
2. **Real-time Consistency**: No duplicate/missing records during pagination
3. **Flexible Filtering**: Status, date range, currency, provider filters
4. **Mathematical Optimization**: Keyset pagination for O(1) performance
5. **Codebase Alignment**: Follow existing patterns (Redis caching, JWT auth, CRUD structure)

---

## 📊 Mathematical Approach: Keyset Pagination

### Why Keyset Over Offset?

**Traditional Offset Pagination (Current Pattern):**
```sql
-- Page 100 with 20 items per page
SELECT * FROM swaps 
WHERE user_id = ? 
ORDER BY created_at DESC 
LIMIT 20 OFFSET 2000;  -- Must scan 2020 rows!
```
- **Performance**: O(N) - degrades linearly with page depth
- **Problem**: At page 100, database scans 2000+ rows just to skip them
- **Instability**: New inserts cause duplicates/gaps between page requests

**Keyset Pagination (Recommended):**
```sql
-- First page
SELECT * FROM swaps 
WHERE user_id = ? 
ORDER BY created_at DESC, id DESC 
LIMIT 21;  -- Fetch 21 to detect "has_more"

-- Next page (cursor = last row's created_at + id)
SELECT * FROM swaps 
WHERE user_id = ? 
  AND (created_at, id) < (?, ?)  -- Cursor values
ORDER BY created_at DESC, id DESC 
LIMIT 21;
```
- **Performance**: O(1) - constant time regardless of page depth
- **Stability**: Immune to concurrent inserts/deletes
- **Index-friendly**: Uses composite index efficiently

### Mathematical Proof of Performance

**Offset Method Complexity:**
```
Time(page_n) = O(page_size × n)
Example: Page 100 with size 20 = O(2000) row scans
```

**Keyset Method Complexity:**
```
Time(page_n) = O(log(total_rows) + page_size)
Example: Page 100 with size 20 = O(log(100000) + 20) ≈ O(37) operations
Speedup: 2000/37 ≈ 54x faster!
```

---

## 🏗️ Architecture Design

### 1. Database Schema (Already Exists)

```sql
-- swaps table (existing)
CREATE TABLE swaps (
    id VARCHAR(36) PRIMARY KEY,
    user_id VARCHAR(36),
    status ENUM(...),
    from_currency VARCHAR(20),
    to_currency VARCHAR(20),
    amount DECIMAL(20, 8),
    estimated_receive DECIMAL(20, 8),
    created_at TIMESTAMP,
    updated_at TIMESTAMP,
    -- ... other fields
    INDEX idx_swaps_user_id (user_id),
    INDEX idx_swaps_created_at (created_at)
);
```

**New Composite Index Required:**
```sql
-- Optimized for keyset pagination
CREATE INDEX idx_swaps_user_history 
ON swaps (user_id, created_at DESC, id DESC)
WHERE user_id IS NOT NULL;

-- For status filtering
CREATE INDEX idx_swaps_user_status_history 
ON swaps (user_id, status, created_at DESC, id DESC)
WHERE user_id IS NOT NULL;
```

### 2. API Endpoint Specification

**Route:** `GET /swap/history`

**Authentication:** Required (JWT via `User` extractor, not `OptionalUser`)

**Query Parameters:**
```typescript
{
  // Pagination (Keyset)
  cursor?: string,           // Base64-encoded cursor from previous response
  limit?: number,            // Default: 20, Max: 100
  
  // Filtering
  status?: string,           // 'waiting' | 'completed' | 'failed' | etc.
  from_currency?: string,    // 'BTC', 'ETH', etc.
  to_currency?: string,      // 'ETH', 'USDT', etc.
  provider?: string,         // 'changenow', 'changelly', etc.
  date_from?: string,        // ISO 8601: '2024-01-01T00:00:00Z'
  date_to?: string,          // ISO 8601: '2024-12-31T23:59:59Z'
  
  // Sorting (optional, default: created_at DESC)
  sort_by?: string,          // 'created_at' | 'amount' | 'status'
  sort_order?: string        // 'asc' | 'desc'
}
```

**Response Format:**
```json
{
  "swaps": [
    {
      "id": "uuid",
      "status": "completed",
      "from_currency": "BTC",
      "from_network": "bitcoin",
      "to_currency": "ETH",
      "to_network": "ethereum",
      "amount": 0.1,
      "estimated_receive": 1.5,
      "actual_receive": 1.48,
      "rate": 15.0,
      "platform_fee": 0.02,
      "total_fee": 0.02,
      "deposit_address": "bc1q...",
      "recipient_address": "0x...",
      "provider": "changenow",
      "created_at": "2024-01-15T10:30:00Z",
      "completed_at": "2024-01-15T10:45:00Z"
    }
  ],
  "pagination": {
    "limit": 20,
    "has_more": true,
    "next_cursor": "eyJjcmVhdGVkX2F0IjoiMjAyNC0wMS0xNVQxMDozMDowMFoiLCJpZCI6InV1aWQifQ=="
  },
  "filters_applied": {
    "status": "completed",
    "date_from": "2024-01-01T00:00:00Z"
  }
}
```

### 3. Cursor Design

**Cursor Structure (JSON before encoding):**
```rust
#[derive(Serialize, Deserialize)]
struct HistoryCursor {
    created_at: DateTime<Utc>,  // Last row's timestamp
    id: String,                  // Last row's ID (tie-breaker)
    
    // Optional: Snapshot filters to detect filter changes
    status: Option<String>,
    from_currency: Option<String>,
    to_currency: Option<String>,
}
```

**Encoding/Decoding:**
```rust
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};

fn encode_cursor(cursor: &HistoryCursor) -> String {
    let json = serde_json::to_string(cursor).unwrap();
    URL_SAFE_NO_PAD.encode(json.as_bytes())
}

fn decode_cursor(cursor_str: &str) -> Result<HistoryCursor, String> {
    let bytes = URL_SAFE_NO_PAD.decode(cursor_str)
        .map_err(|e| format!("Invalid cursor: {}", e))?;
    let json = String::from_utf8(bytes)
        .map_err(|e| format!("Invalid cursor encoding: {}", e))?;
    serde_json::from_str(&json)
        .map_err(|e| format!("Invalid cursor format: {}", e))
}
```

---

## 🔧 Implementation Plan

### Phase 1: Schema & Models (30 min)

**File:** `src/modules/swap/schema.rs`

```rust
// Add to existing schema.rs

#[derive(Debug, Deserialize)]
pub struct HistoryQuery {
    // Pagination
    pub cursor: Option<String>,
    #[serde(default = "default_limit")]
    pub limit: u32,
    
    // Filters
    pub status: Option<String>,
    pub from_currency: Option<String>,
    pub to_currency: Option<String>,
    pub provider: Option<String>,
    pub date_from: Option<String>,  // ISO 8601
    pub date_to: Option<String>,    // ISO 8601
    
    // Sorting
    pub sort_by: Option<String>,
    pub sort_order: Option<String>,
}

fn default_limit() -> u32 { 20 }

#[derive(Debug, Serialize, Deserialize)]
pub struct HistoryCursor {
    pub created_at: DateTime<Utc>,
    pub id: String,
    
    // Filter snapshot for validation
    pub status: Option<String>,
    pub from_currency: Option<String>,
    pub to_currency: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct HistoryResponse {
    pub swaps: Vec<SwapSummary>,
    pub pagination: PaginationInfo,
    pub filters_applied: FiltersApplied,
}

#[derive(Debug, Serialize)]
pub struct SwapSummary {
    pub id: String,
    pub status: SwapStatus,
    pub from_currency: String,
    pub from_network: String,
    pub to_currency: String,
    pub to_network: String,
    pub amount: f64,
    pub estimated_receive: f64,
    pub actual_receive: Option<f64>,
    pub rate: f64,
    pub platform_fee: f64,
    pub total_fee: f64,
    pub deposit_address: String,
    pub recipient_address: String,
    pub provider: String,
    pub created_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize)]
pub struct PaginationInfo {
    pub limit: u32,
    pub has_more: bool,
    pub next_cursor: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct FiltersApplied {
    pub status: Option<String>,
    pub from_currency: Option<String>,
    pub to_currency: Option<String>,
    pub provider: Option<String>,
    pub date_from: Option<String>,
    pub date_to: Option<String>,
}
```

### Phase 2: CRUD Implementation (1-2 hours)

**File:** `src/modules/swap/crud.rs`

```rust
impl SwapCrud {
    /// Get user's swap history with keyset pagination
    pub async fn get_swap_history(
        &self,
        user_id: &str,
        query: HistoryQuery,
    ) -> Result<HistoryResponse, SwapError> {
        // 1. Validate and parse cursor
        let cursor = if let Some(cursor_str) = &query.cursor {
            Some(self.decode_cursor(cursor_str)?)
        } else {
            None
        };
        
        // 2. Validate limit
        let limit = query.limit.min(100).max(1);
        
        // 3. Parse date filters
        let date_from = query.date_from.as_ref()
            .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
            .map(|dt| dt.with_timezone(&Utc));
        let date_to = query.date_to.as_ref()
            .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
            .map(|dt| dt.with_timezone(&Utc));
        
        // 4. Build dynamic SQL query with keyset pagination
        let mut sql = String::from(
            "SELECT 
                id, user_id, provider_id, status,
                from_currency, from_network, to_currency, to_network,
                CAST(amount AS DOUBLE) as amount,
                CAST(estimated_receive AS DOUBLE) as estimated_receive,
                CAST(actual_receive AS DOUBLE) as actual_receive,
                CAST(rate AS DOUBLE) as rate,
                CAST(platform_fee AS DOUBLE) as platform_fee,
                CAST(total_fee AS DOUBLE) as total_fee,
                deposit_address, recipient_address,
                created_at, completed_at
            FROM swaps
            WHERE user_id = ?"
        );
        
        let mut conditions = Vec::new();
        
        // 5. Apply keyset cursor (most important for performance!)
        if let Some(ref c) = cursor {
            conditions.push(format!(
                "(created_at, id) < ('{}', '{}')",
                c.created_at.to_rfc3339(),
                c.id
            ));
        }
        
        // 6. Apply filters
        if let Some(ref status) = query.status {
            conditions.push(format!("status = '{}'", status.replace("'", "''")));
        }
        if let Some(ref from) = query.from_currency {
            conditions.push(format!("from_currency = '{}'", from.replace("'", "''")));
        }
        if let Some(ref to) = query.to_currency {
            conditions.push(format!("to_currency = '{}'", to.replace("'", "''")));
        }
        if let Some(ref provider) = query.provider {
            conditions.push(format!("provider_id = '{}'", provider.replace("'", "''")));
        }
        if let Some(dt) = date_from {
            conditions.push(format!("created_at >= '{}'", dt.to_rfc3339()));
        }
        if let Some(dt) = date_to {
            conditions.push(format!("created_at <= '{}'", dt.to_rfc3339()));
        }
        
        if !conditions.is_empty() {
            sql.push_str(" AND ");
            sql.push_str(&conditions.join(" AND "));
        }
        
        // 7. Apply sorting (default: created_at DESC, id DESC)
        let sort_by = query.sort_by.as_deref().unwrap_or("created_at");
        let sort_order = query.sort_order.as_deref().unwrap_or("desc");
        sql.push_str(&format!(
            " ORDER BY {} {}, id {}",
            sort_by, sort_order.to_uppercase(), sort_order.to_uppercase()
        ));
        
        // 8. Fetch limit + 1 to detect "has_more"
        sql.push_str(&format!(" LIMIT {}", limit + 1));
        
        // 9. Execute query
        let rows = sqlx::query(&sql)
            .bind(user_id)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| SwapError::DatabaseError(e.to_string()))?;
        
        // 10. Process results
        let has_more = rows.len() > limit as usize;
        let swaps_data = if has_more {
            &rows[..limit as usize]
        } else {
            &rows[..]
        };
        
        // 11. Map to SwapSummary
        let swaps: Vec<SwapSummary> = swaps_data.iter().map(|row| {
            SwapSummary {
                id: row.get("id"),
                status: row.get("status"),
                from_currency: row.get("from_currency"),
                from_network: row.get("from_network"),
                to_currency: row.get("to_currency"),
                to_network: row.get("to_network"),
                amount: row.get("amount"),
                estimated_receive: row.get("estimated_receive"),
                actual_receive: row.get("actual_receive"),
                rate: row.get("rate"),
                platform_fee: row.get("platform_fee"),
                total_fee: row.get("total_fee"),
                deposit_address: row.get("deposit_address"),
                recipient_address: row.get("recipient_address"),
                provider: row.get("provider_id"),
                created_at: row.get("created_at"),
                completed_at: row.get("completed_at"),
            }
        }).collect();
        
        // 12. Generate next cursor
        let next_cursor = if has_more && !swaps.is_empty() {
            let last = &swaps[swaps.len() - 1];
            Some(self.encode_cursor(&HistoryCursor {
                created_at: last.created_at,
                id: last.id.clone(),
                status: query.status.clone(),
                from_currency: query.from_currency.clone(),
                to_currency: query.to_currency.clone(),
            }))
        } else {
            None
        };
        
        // 13. Build response
        Ok(HistoryResponse {
            swaps,
            pagination: PaginationInfo {
                limit,
                has_more,
                next_cursor,
            },
            filters_applied: FiltersApplied {
                status: query.status,
                from_currency: query.from_currency,
                to_currency: query.to_currency,
                provider: query.provider,
                date_from: query.date_from,
                date_to: query.date_to,
            },
        })
    }
    
    /// Encode cursor to base64
    fn encode_cursor(&self, cursor: &HistoryCursor) -> String {
        use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
        let json = serde_json::to_string(cursor).unwrap();
        URL_SAFE_NO_PAD.encode(json.as_bytes())
    }
    
    /// Decode cursor from base64
    fn decode_cursor(&self, cursor_str: &str) -> Result<HistoryCursor, SwapError> {
        use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
        
        let bytes = URL_SAFE_NO_PAD.decode(cursor_str)
            .map_err(|_| SwapError::DatabaseError("Invalid cursor format".to_string()))?;
        let json = String::from_utf8(bytes)
            .map_err(|_| SwapError::DatabaseError("Invalid cursor encoding".to_string()))?;
        serde_json::from_str(&json)
            .map_err(|_| SwapError::DatabaseError("Invalid cursor structure".to_string()))
    }
}
```

### Phase 3: Controller Implementation (30 min)

**File:** `src/modules/swap/controller.rs`

```rust
use crate::modules::auth::interface::User;  // Not OptionalUser!

/// GET /swap/history - Get authenticated user's swap history
pub async fn get_swap_history(
    State(state): State<Arc<AppState>>,
    user: User,  // Requires authentication
    Query(query): Query<HistoryQuery>,
) -> Result<Json<HistoryResponse>, (StatusCode, Json<SwapErrorResponse>)> {
    let crud = SwapCrud::new(
        state.db.clone(),
        Some(state.redis.clone()),
        Some(state.wallet_mnemonic.clone())
    );
    
    let response = crud.get_swap_history(&user.id, query).await.map_err(|e| {
        let status = match e {
            SwapError::DatabaseError(_) => StatusCode::INTERNAL_SERVER_ERROR,
            _ => StatusCode::BAD_REQUEST,
        };
        (status, Json(SwapErrorResponse::new(e.to_string())))
    })?;
    
    Ok(Json(response))
}
```

### Phase 4: Route Registration (5 min)

**File:** `src/modules/swap/routes.rs`

```rust
use super::controller::{
    get_currencies, get_providers, get_rates, 
    create_swap, get_swap_status, validate_address,
    get_swap_history  // Add this
};

pub fn swap_routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/currencies", get(get_currencies))
        .route("/providers", get(get_providers))
        .route("/rates", get(get_rates))
        .route("/create", post(create_swap))
        .route("/{id}", get(get_swap_status))
        .route("/validate-address", post(validate_address))
        .route("/history", get(get_swap_history))  // Add this
}
```

### Phase 5: Database Migration (5 min)

**File:** `migrations/20260218000001_add_swap_history_indexes.sql`

```sql
-- ============================================================================
-- Migration: Add optimized indexes for swap history pagination
-- Created: 2026-02-18
-- Description: Composite indexes for keyset pagination performance
-- ============================================================================

-- Primary index for keyset pagination (user + time + id)
CREATE INDEX IF NOT EXISTS idx_swaps_user_history 
ON swaps (user_id, created_at DESC, id DESC)
WHERE user_id IS NOT NULL;

-- Index for status filtering
CREATE INDEX IF NOT EXISTS idx_swaps_user_status_history 
ON swaps (user_id, status, created_at DESC, id DESC)
WHERE user_id IS NOT NULL;

-- Index for currency filtering
CREATE INDEX IF NOT EXISTS idx_swaps_user_currency_history 
ON swaps (user_id, from_currency, to_currency, created_at DESC, id DESC)
WHERE user_id IS NOT NULL;

-- Index for provider filtering
CREATE INDEX IF NOT EXISTS idx_swaps_user_provider_history 
ON swaps (user_id, provider_id, created_at DESC, id DESC)
WHERE user_id IS NOT NULL;
```

---

## 🚀 Performance Optimizations

### 1. Redis Caching Strategy

**Cache Key Pattern:**
```
swap_history:{user_id}:{filters_hash}:{cursor}
```

**Implementation:**
```rust
// In get_swap_history(), before database query:
let cache_key = format!(
    "swap_history:{}:{}:{}",
    user_id,
    self.hash_filters(&query),
    query.cursor.as_deref().unwrap_or("first")
);

// Try cache first (TTL: 60 seconds for recent data)
if let Some(cached) = self.redis_service.as_ref()
    .and_then(|r| r.get_json::<HistoryResponse>(&cache_key).await.ok().flatten()) 
{
    return Ok(cached);
}

// ... execute query ...

// Cache result
if let Some(redis) = &self.redis_service {
    let _ = redis.set_json(&cache_key, &response, 60).await;
}
```

### 2. Query Optimization Tips

**Index Usage Verification:**
```sql
EXPLAIN SELECT * FROM swaps 
WHERE user_id = 'uuid' 
  AND (created_at, id) < ('2024-01-15 10:30:00', 'uuid')
ORDER BY created_at DESC, id DESC 
LIMIT 21;

-- Should show: "Using index idx_swaps_user_history"
```

**Covering Index (Advanced):**
```sql
-- Include frequently accessed columns in index
CREATE INDEX idx_swaps_user_history_covering 
ON swaps (
    user_id, created_at DESC, id DESC,
    status, from_currency, to_currency, amount
)
WHERE user_id IS NOT NULL;
```

### 3. Pagination Best Practices

**Client-Side Implementation:**
```javascript
// Frontend example
let cursor = null;
const allSwaps = [];

while (true) {
    const response = await fetch(
        `/swap/history?limit=50${cursor ? `&cursor=${cursor}` : ''}`
    );
    const data = await response.json();
    
    allSwaps.push(...data.swaps);
    
    if (!data.pagination.has_more) break;
    cursor = data.pagination.next_cursor;
}
```

---

## 🧪 Testing Strategy

### Unit Tests

**File:** `tests/swap/history_test.rs`

```rust
#[tokio::test]
async fn test_history_first_page() {
    // Test fetching first page without cursor
}

#[tokio::test]
async fn test_history_keyset_pagination() {
    // Test cursor-based pagination consistency
}

#[tokio::test]
async fn test_history_with_filters() {
    // Test status, currency, date filters
}

#[tokio::test]
async fn test_history_concurrent_inserts() {
    // Verify no duplicates when new swaps created during pagination
}

#[tokio::test]
async fn test_history_invalid_cursor() {
    // Test error handling for malformed cursors
}

#[tokio::test]
async fn test_history_authentication_required() {
    // Verify endpoint requires valid JWT
}
```

### Performance Benchmarks

```rust
#[tokio::test]
async fn benchmark_history_deep_pagination() {
    // Create 100K swaps for user
    // Measure time for page 1, 100, 1000
    // Assert: All pages < 100ms
}
```

---

## 📈 Monitoring & Metrics

### Key Metrics to Track

1. **Query Performance:**
   - P50, P95, P99 latency per page depth
   - Index hit rate
   - Cache hit rate

2. **Usage Patterns:**
   - Most common filters
   - Average page depth
   - Cursor reuse rate

3. **Error Rates:**
   - Invalid cursor errors
   - Timeout errors
   - Authentication failures

### Logging

```rust
tracing::info!(
    user_id = %user_id,
    filters = ?query,
    page_depth = cursor.is_some(),
    result_count = swaps.len(),
    has_more = has_more,
    duration_ms = start.elapsed().as_millis(),
    "Swap history fetched"
);
```

---

## 🔒 Security Considerations

1. **Authorization:** User can only see their own swaps (enforced by `user_id` filter)
2. **Rate Limiting:** Apply existing rate limiter to prevent abuse
3. **Cursor Validation:** Decode and validate cursor structure
4. **SQL Injection:** Use parameterized queries (already done via sqlx)
5. **PII Protection:** Don't log sensitive data (addresses, amounts)

---

## 📝 API Documentation Example

```yaml
/swap/history:
  get:
    summary: Get user's swap history
    security:
      - bearerAuth: []
    parameters:
      - name: cursor
        in: query
        schema:
          type: string
        description: Pagination cursor from previous response
      - name: limit
        in: query
        schema:
          type: integer
          minimum: 1
          maximum: 100
          default: 20
      - name: status
        in: query
        schema:
          type: string
          enum: [waiting, confirming, exchanging, sending, completed, failed]
      - name: from_currency
        in: query
        schema:
          type: string
      - name: date_from
        in: query
        schema:
          type: string
          format: date-time
    responses:
      200:
        description: Swap history retrieved successfully
        content:
          application/json:
            schema:
              $ref: '#/components/schemas/HistoryResponse'
      401:
        description: Unauthorized - Invalid or missing JWT token
      400:
        description: Bad Request - Invalid cursor or parameters
```

---

## ✅ Implementation Checklist

- [ ] Add `HistoryQuery`, `HistoryResponse`, `HistoryCursor` to `schema.rs`
- [ ] Implement `get_swap_history()` in `crud.rs` with keyset pagination
- [ ] Add cursor encoding/decoding helpers
- [ ] Implement `get_swap_history()` controller in `controller.rs`
- [ ] Register route in `routes.rs`
- [ ] Create database migration for composite indexes
- [ ] Run migration: `sqlx migrate run`
- [ ] Add Redis caching layer
- [ ] Write unit tests in `tests/swap/history_test.rs`
- [ ] Test with 10K+ swaps for performance validation
- [ ] Verify index usage with `EXPLAIN`
- [ ] Add monitoring/logging
- [ ] Update API documentation
- [ ] Test authentication enforcement
- [ ] Test cursor edge cases (invalid, expired, filter mismatch)

---

## 🎓 Key Takeaways

1. **Keyset pagination is 50-100x faster** than offset for deep pages
2. **Composite indexes are critical** - (user_id, created_at DESC, id DESC)
3. **Cursor must include tie-breaker** (id) to prevent duplicates
4. **Fetch limit+1** to detect has_more without extra query
5. **Cache aggressively** but with short TTL (60s) for consistency
6. **Always use DESC ordering** for time-series data (newest first)
7. **Validate cursors** to prevent injection/tampering

---

## 📚 References

- [Keyset Pagination Performance Analysis](https://www.caduh.com/blog/pagination-that-scales-offset-cursor-keyset)
- [MySQL Keyset Optimization](https://openillumi.com/en/en-mysql-pagination-fix-keyset-speedup/)
- [PostgreSQL Seek Method](https://blog.jooq.org/2013/10/26/faster-sql-paging-with-jooq-using-the-seek-method/)
- Existing codebase patterns: `swap/crud.rs`, `auth/controller.rs`

---

**Estimated Implementation Time:** 3-4 hours
**Performance Gain:** 50-100x for deep pagination
**Complexity:** Medium (follows existing patterns)

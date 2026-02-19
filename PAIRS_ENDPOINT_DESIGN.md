# Trading Pairs Endpoint - Design & Implementation Guide

## Research Summary

Based on industry best practices from major cryptocurrency exchanges (Nexchange, Kraken, Binance, Alpaca) and REST API design standards, here's the optimal approach for implementing the trading pairs endpoint.

## Mathematical & Algorithmic Approaches

### 1. Pair Availability Calculation
**Formula**: A trading pair is available if:
```
available = (base_currency.enabled AND quote_currency.enabled) 
            AND (liquidity_score > threshold)
            AND (last_price_update < max_staleness)
```

### 2. Liquidity Score Calculation
**Amihud Ratio** (Low-frequency liquidity proxy):
```
Liquidity_Score = 1 / Amihud_Ratio
Amihud_Ratio = |Return| / Volume

Where:
- Return = (Price_t - Price_t-1) / Price_t-1
- Volume = Trading volume in USD over period
```

**Interpretation**:
- Higher Amihud Ratio = Lower liquidity (price moves more per dollar traded)
- Lower Amihud Ratio = Higher liquidity (price stable despite volume)

### 3. Pair Ranking Algorithm
**Composite Score** for "most_traded" or featured pairs:
```
Pair_Score = w1 * Volume_24h_normalized 
           + w2 * Trade_Count_normalized
           + w3 * Liquidity_Score_normalized
           + w4 * Spread_Score

Where:
- w1, w2, w3, w4 are weights (sum to 1.0)
- Recommended: w1=0.4, w2=0.3, w3=0.2, w4=0.1
- Spread_Score = 1 / (bid_ask_spread_percentage)
```

### 4. Pagination Mathematics
**Offset-based Pagination**:
```
Total_Pages = CEIL(Total_Items / Page_Size)
Offset = (Page_Number - 1) * Page_Size
LIMIT = Page_Size
```

**Cursor-based Pagination** (Recommended for large datasets):
```
WHERE id > cursor_value
ORDER BY id ASC
LIMIT page_size
```

## REST API Design

### Endpoint Structure

#### 1. List All Trading Pairs
```http
GET /api/v1/swap/pairs
```

**Query Parameters**:
- `page` (integer, default: 0) - Page number (0-indexed)
- `size` (integer, default: 20, max: 100) - Items per page
- `orderBy` (string, default: "volume_desc") - Sort field and direction
- `filter` (string, optional) - Filter expression
- `status` (string, optional) - Filter by status: "active", "disabled", "all"

**Example Requests**:
```http
# Basic request
GET /api/v1/swap/pairs?page=0&size=20

# With sorting
GET /api/v1/swap/pairs?page=0&size=20&orderBy=volume desc,name asc

# With filtering
GET /api/v1/swap/pairs?filter=base_currency eq 'BTC' and status eq 'active'

# Active pairs only
GET /api/v1/swap/pairs?status=active&orderBy=volume desc
```

**Response Structure**:
```json
{
  "pairs": [
    {
      "name": "BTC/USDT",
      "base_currency": "BTC",
      "quote_currency": "USDT",
      "base_network": "bitcoin",
      "quote_network": "ethereum",
      "status": "active",
      "volume_24h": 1234567.89,
      "trade_count_24h": 5432,
      "liquidity_score": 0.95,
      "last_price": 45000.00,
      "price_change_24h": 2.5,
      "bid_ask_spread": 0.05,
      "min_amount": 0.0001,
      "max_amount": 100.0,
      "last_updated": "2026-02-19T10:30:00Z",
      "_links": {
        "self": {
          "href": "/api/v1/swap/pairs/BTC-USDT"
        },
        "estimate": {
          "href": "/api/v1/swap/estimate?from=BTC&to=USDT"
        }
      }
    }
  ],
  "_links": {
    "self": {
      "href": "/api/v1/swap/pairs?page=0&size=20"
    },
    "first": {
      "href": "/api/v1/swap/pairs?page=0&size=20"
    },
    "next": {
      "href": "/api/v1/swap/pairs?page=1&size=20"
    },
    "last": {
      "href": "/api/v1/swap/pairs?page=5&size=20"
    }
  },
  "page": {
    "size": 20,
    "totalElements": 120,
    "totalPages": 6,
    "number": 0
  }
}
```

**Response Headers**:
```http
HTTP/1.1 200 OK
Content-Type: application/json
Link: <http://localhost:8080/api/v1/swap/pairs?page=0&size=20>; rel="self"
Link: <http://localhost:8080/api/v1/swap/pairs?page=0&size=20>; rel="first"
Link: <http://localhost:8080/api/v1/swap/pairs?page=1&size=20>; rel="next"
Link: <http://localhost:8080/api/v1/swap/pairs?page=5&size=20>; rel="last"
X-Total-Items: 120
X-Page: 0
X-Page-Size: 20
X-Total-Pages: 6
```

#### 2. Get Specific Pair Details
```http
GET /api/v1/swap/pairs/{pair_name}
```

**Path Parameters**:
- `pair_name` (string) - Pair identifier (e.g., "BTC-USDT", "ETH-BTC")

**Example**:
```http
GET /api/v1/swap/pairs/BTC-USDT
```

**Response**:
```json
{
  "name": "BTC/USDT",
  "base_currency": "BTC",
  "quote_currency": "USDT",
  "base_network": "bitcoin",
  "quote_network": "ethereum",
  "status": "active",
  "volume_24h": 1234567.89,
  "trade_count_24h": 5432,
  "liquidity_score": 0.95,
  "last_price": 45000.00,
  "price_change_24h": 2.5,
  "bid_ask_spread": 0.05,
  "min_amount": 0.0001,
  "max_amount": 100.0,
  "fee_percentage": 0.5,
  "supported_networks": {
    "BTC": ["bitcoin", "lightning"],
    "USDT": ["ethereum", "tron", "bsc"]
  },
  "last_updated": "2026-02-19T10:30:00Z",
  "_links": {
    "self": {
      "href": "/api/v1/swap/pairs/BTC-USDT"
    },
    "estimate": {
      "href": "/api/v1/swap/estimate?from=BTC&to=USDT"
    },
    "create_swap": {
      "href": "/api/v1/swap"
    }
  }
}
```

#### 3. Special Endpoints (Optional)

**Get Random Active Pair**:
```http
GET /api/v1/swap/pairs/random
```

**Get Most Traded Pair**:
```http
GET /api/v1/swap/pairs/most_traded
```

**Get Featured Pairs**:
```http
GET /api/v1/swap/pairs/featured
```

## Filtering Operators

Based on [RFC-8040](https://www.rfc-editor.org/rfc/rfc8040.html) and Microsoft API Guidelines:

| Operator | Description | Example |
|----------|-------------|---------|
| `eq` | Equal | `base_currency eq 'BTC'` |
| `ne` | Not equal | `status ne 'disabled'` |
| `gt` | Greater than | `volume_24h gt 100000` |
| `ge` | Greater than or equal | `liquidity_score ge 0.8` |
| `lt` | Less than | `bid_ask_spread lt 0.1` |
| `le` | Less than or equal | `fee_percentage le 1.0` |
| `and` | Logical AND | `status eq 'active' and volume_24h gt 10000` |
| `or` | Logical OR | `base_currency eq 'BTC' or base_currency eq 'ETH'` |
| `not` | Logical negation | `not status eq 'disabled'` |
| `()` | Precedence grouping | `(base_currency eq 'BTC' or base_currency eq 'ETH') and status eq 'active'` |

**Complex Filter Example**:
```http
GET /api/v1/swap/pairs?filter=(base_currency eq 'BTC' or base_currency eq 'ETH') and volume_24h gt 50000 and status eq 'active'
```

## Sorting Options

**Supported Sort Fields**:
- `name` - Pair name (alphabetical)
- `volume_24h` - 24-hour trading volume
- `trade_count_24h` - Number of trades
- `liquidity_score` - Liquidity rating
- `price_change_24h` - Price change percentage
- `last_updated` - Last update timestamp

**Sort Direction**:
- `asc` - Ascending order
- `desc` - Descending order (default)
- `-` prefix - Descending (alternative syntax)

**Examples**:
```http
# Sort by volume descending
GET /api/v1/swap/pairs?orderBy=volume_24h desc

# Multiple sort fields
GET /api/v1/swap/pairs?orderBy=volume_24h desc,name asc

# Using hyphen syntax
GET /api/v1/swap/pairs?orderBy=-volume_24h,name
```

## Database Schema Considerations

### Option 1: Dedicated trading_pairs Table
```sql
CREATE TABLE trading_pairs (
    id BIGINT PRIMARY KEY AUTO_INCREMENT,
    name VARCHAR(20) NOT NULL UNIQUE,
    base_currency_id INT NOT NULL,
    quote_currency_id INT NOT NULL,
    base_network VARCHAR(50),
    quote_network VARCHAR(50),
    status ENUM('active', 'disabled', 'maintenance') DEFAULT 'active',
    volume_24h DECIMAL(20, 8) DEFAULT 0,
    trade_count_24h INT DEFAULT 0,
    liquidity_score DECIMAL(5, 4) DEFAULT 0,
    last_price DECIMAL(20, 8),
    price_change_24h DECIMAL(10, 4),
    bid_ask_spread DECIMAL(10, 6),
    min_amount DECIMAL(20, 8),
    max_amount DECIMAL(20, 8),
    fee_percentage DECIMAL(5, 4),
    last_updated TIMESTAMP DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    
    FOREIGN KEY (base_currency_id) REFERENCES currencies(id),
    FOREIGN KEY (quote_currency_id) REFERENCES currencies(id),
    INDEX idx_status (status),
    INDEX idx_volume (volume_24h DESC),
    INDEX idx_liquidity (liquidity_score DESC),
    INDEX idx_updated (last_updated DESC)
);
```

### Option 2: Derive from Currencies (Dynamic)
```sql
-- Query to generate pairs dynamically
SELECT 
    CONCAT(c1.ticker, '/', c2.ticker) as name,
    c1.ticker as base_currency,
    c2.ticker as quote_currency,
    c1.network as base_network,
    c2.network as quote_network,
    CASE 
        WHEN c1.enabled = 1 AND c2.enabled = 1 THEN 'active'
        ELSE 'disabled'
    END as status
FROM currencies c1
CROSS JOIN currencies c2
WHERE c1.id != c2.id
    AND c1.enabled = 1
    AND c2.enabled = 1
ORDER BY c1.ticker, c2.ticker;
```

**Recommendation**: Use dedicated `trading_pairs` table for:
- Better performance (pre-computed metrics)
- Ability to disable specific pairs
- Store pair-specific metadata (fees, limits)
- Track historical metrics

## Performance Optimizations

### 1. Caching Strategy
```rust
// Multi-tier caching
- L1: In-memory cache (10s TTL) for hot pairs
- L2: Redis cache (60s TTL) for all pairs
- L3: Database with proper indexes
```

### 2. Database Indexes
```sql
-- Composite index for common queries
CREATE INDEX idx_status_volume ON trading_pairs(status, volume_24h DESC);

-- Covering index for list queries
CREATE INDEX idx_pairs_list ON trading_pairs(
    status, volume_24h DESC, name
) INCLUDE (base_currency_id, quote_currency_id, last_price);
```

### 3. Query Optimization
```sql
-- Use LIMIT with proper offset
SELECT * FROM trading_pairs
WHERE status = 'active'
ORDER BY volume_24h DESC
LIMIT 20 OFFSET 0;

-- For cursor-based pagination
SELECT * FROM trading_pairs
WHERE status = 'active' AND id > ?
ORDER BY id ASC
LIMIT 20;
```

## Error Responses

### 400 Bad Request - Invalid Filter
```json
{
  "error": {
    "code": "InvalidFilterExpression",
    "message": "The property 'invalid_field' in the filter expression is not supported",
    "details": [
      {
        "code": "UnsupportedFilterProperty",
        "target": "invalid_field",
        "message": "Filtering by 'invalid_field' is not supported. Supported properties are: ['base_currency', 'quote_currency', 'status', 'volume_24h']"
      }
    ],
    "target": "filter"
  }
}
```

### 400 Bad Request - Invalid Sort Field
```json
{
  "error": {
    "code": "InvalidOrderByExpression",
    "message": "The property 'invalid_field' in the orderby expression is not sortable",
    "details": [
      {
        "code": "UnsupportedSortProperty",
        "target": "invalid_field",
        "message": "Sorting by 'invalid_field' is not supported. Supported sort properties are: ['name', 'volume_24h', 'liquidity_score', 'last_updated']"
      }
    ],
    "target": "orderby"
  }
}
```

### 404 Not Found - Pair Not Found
```json
{
  "error": {
    "code": "PairNotFound",
    "message": "Trading pair 'BTC-XYZ' not found",
    "target": "pair_name"
  }
}
```

## Implementation Checklist

- [ ] Create/update `trading_pairs` database table
- [ ] Implement CRUD operations in `PairsCrud`
- [ ] Create `PairsController` with GET endpoints
- [ ] Add pagination support (offset-based)
- [ ] Implement filtering with operator parsing
- [ ] Add sorting with multiple fields
- [ ] Implement caching layer (Redis)
- [ ] Add Link headers for pagination
- [ ] Create response DTOs with HATEOAS links
- [ ] Write comprehensive tests
- [ ] Add API documentation
- [ ] Implement rate limiting
- [ ] Add monitoring/metrics

## Testing Strategy

### Unit Tests
- Pagination calculation
- Filter expression parsing
- Sort field validation
- Pair availability logic

### Integration Tests
- List pairs with pagination
- Filter by various criteria
- Sort by different fields
- Get specific pair details
- Handle invalid requests

### Performance Tests
- Load test with 1000+ pairs
- Concurrent request handling
- Cache hit rate measurement
- Query performance benchmarks

## References

Content rephrased for compliance with licensing restrictions:

- REST API pagination patterns from [restfulapi.net](https://restfulapi.net/api-pagination-sorting-filtering/)
- Trading pairs design from [Nexchange API](https://docs.n.exchange/docs/current/api-reference/pairs)
- Query parameter standards from [RFC-8040](https://www.rfc-editor.org/rfc/rfc8040.html)
- Filtering operators from [Microsoft API Guidelines](https://github.com/microsoft/api-guidelines/)
- Liquidity metrics from academic research on cryptocurrency market microstructure

## Next Steps

1. Review existing `pairs_test.rs` to understand test requirements
2. Design database schema (dedicated table vs derived)
3. Implement basic endpoint without filtering/sorting
4. Add pagination support
5. Implement filtering and sorting
6. Add caching layer
7. Write comprehensive tests
8. Document API endpoints

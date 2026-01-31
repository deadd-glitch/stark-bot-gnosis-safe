---
name: polymarket
description: "Interact with Polymarket prediction markets - browse markets, get prices, orderbooks, and market data."
version: 1.0.0
author: starkbot
homepage: https://docs.polymarket.com/
metadata: {"clawdbot":{"emoji":"📊"}}
requires_tools: [web_fetch]
tags: [polymarket, prediction-markets, trading, crypto, defi, markets, prices]
arguments:
  market_id:
    description: "Polymarket market/token ID (condition_id or token_id)"
    required: false
  event_slug:
    description: "Event slug for looking up specific events (e.g., 'presidential-election-winner-2024')"
    required: false
  search_query:
    description: "Search term to find markets"
    required: false
---

# Polymarket Operations Guide

You have access to Polymarket's prediction market data through the `web_fetch` tool. Polymarket is a decentralized prediction market platform on the Polygon blockchain.

## API Base URLs

| API | Base URL | Purpose |
|-----|----------|---------|
| **Gamma** | `https://gamma-api.polymarket.com` | Market discovery, metadata, events |
| **CLOB** | `https://clob.polymarket.com` | Prices, orderbooks, trading |
| **Data** | `https://data-api.polymarket.com` | User positions, activity, history |

---

## Common Operations

### 1. Search/Browse Markets

To find prediction markets on a topic:

```json
{
  "tool": "web_fetch",
  "url": "https://gamma-api.polymarket.com/markets?limit=10&active=true",
  "extract_mode": "raw"
}
```

With search filter:
```json
{
  "tool": "web_fetch",
  "url": "https://gamma-api.polymarket.com/markets?limit=10&active=true&_q={{search_query}}",
  "extract_mode": "raw"
}
```

### 2. Get Market Details by Slug

To get details for a specific event/market:

```json
{
  "tool": "web_fetch",
  "url": "https://gamma-api.polymarket.com/events?slug={{event_slug}}",
  "extract_mode": "raw"
}
```

### 3. Get All Events (Active Markets)

List all active events with their markets:

```json
{
  "tool": "web_fetch",
  "url": "https://gamma-api.polymarket.com/events?active=true&limit=20",
  "extract_mode": "raw"
}
```

### 4. Get Single Event Details

Get detailed information about a specific event by ID:

```json
{
  "tool": "web_fetch",
  "url": "https://gamma-api.polymarket.com/events/{event_id}",
  "extract_mode": "raw"
}
```

---

## Price & Orderbook Data (CLOB API)

### 5. Get Current Price

Get the current price for a market token:

```json
{
  "tool": "web_fetch",
  "url": "https://clob.polymarket.com/price?token_id={{market_id}}&side=buy",
  "extract_mode": "raw"
}
```

Parameters:
- `token_id`: The condition/token ID of the market
- `side`: `buy` or `sell`

### 6. Get Midpoint Price

Get the midpoint between best bid and ask:

```json
{
  "tool": "web_fetch",
  "url": "https://clob.polymarket.com/midpoint?token_id={{market_id}}",
  "extract_mode": "raw"
}
```

### 7. Get Orderbook

View the full orderbook for a market:

```json
{
  "tool": "web_fetch",
  "url": "https://clob.polymarket.com/book?token_id={{market_id}}",
  "extract_mode": "raw"
}
```

### 8. Get Spread

Get bid-ask spread:

```json
{
  "tool": "web_fetch",
  "url": "https://clob.polymarket.com/spread?token_id={{market_id}}",
  "extract_mode": "raw"
}
```

---

## Market Discovery Workflows

### Workflow A: Find Markets on a Topic

1. **Search for markets by keyword:**
```json
{
  "tool": "web_fetch",
  "url": "https://gamma-api.polymarket.com/events?active=true&limit=10&tag=politics",
  "extract_mode": "raw"
}
```

Common tags: `politics`, `crypto`, `sports`, `finance`, `science`, `entertainment`, `world`

2. **Parse the response** to extract:
   - `id`: Event ID
   - `slug`: URL-friendly identifier
   - `title`: Human-readable title
   - `markets`: Array of associated markets with their token IDs

3. **Get prices** for interesting markets using the CLOB API.

### Workflow B: Monitor a Specific Market

1. **Find the market** by slug or search
2. **Extract the token_id** from the market data
3. **Get current price:**
```json
{
  "tool": "web_fetch",
  "url": "https://clob.polymarket.com/price?token_id=<TOKEN_ID>&side=buy",
  "extract_mode": "raw"
}
```

4. **Get orderbook depth:**
```json
{
  "tool": "web_fetch",
  "url": "https://clob.polymarket.com/book?token_id=<TOKEN_ID>",
  "extract_mode": "raw"
}
```

---

## Understanding Market Data

### Event Structure
```json
{
  "id": "event-uuid",
  "slug": "event-name-slug",
  "title": "Will X happen?",
  "description": "Full description...",
  "active": true,
  "closed": false,
  "markets": [
    {
      "id": "market-uuid",
      "question": "Yes outcome",
      "conditionId": "0x...",  // Use this as token_id for CLOB
      "outcomes": ["Yes", "No"],
      "outcomePrices": ["0.65", "0.35"]
    }
  ]
}
```

### Price Interpretation
- Prices are between 0 and 1 (representing probability/percentage)
- Price of 0.65 = 65% implied probability
- Binary markets have YES/NO outcomes where YES + NO prices should sum to ~1

### Orderbook Structure
```json
{
  "bids": [{"price": "0.64", "size": "1000"}, ...],
  "asks": [{"price": "0.66", "size": "500"}, ...]
}
```

---

## Filtering & Pagination

### Query Parameters for Gamma API

| Parameter | Description | Example |
|-----------|-------------|---------|
| `active` | Filter by active status | `active=true` |
| `closed` | Filter by closed status | `closed=false` |
| `limit` | Max results to return | `limit=20` |
| `offset` | Skip N results | `offset=10` |
| `tag` | Filter by category tag | `tag=politics` |
| `_q` | Full-text search | `_q=election` |
| `slug` | Exact slug match | `slug=event-name` |

### Example: Paginated Market List
```json
{
  "tool": "web_fetch",
  "url": "https://gamma-api.polymarket.com/events?active=true&limit=10&offset=0",
  "extract_mode": "raw"
}
```

---

## Quick Reference

### Get Popular/Trending Markets
```json
{
  "tool": "web_fetch",
  "url": "https://gamma-api.polymarket.com/events?active=true&limit=10&order=volume&ascending=false",
  "extract_mode": "raw"
}
```

### Get Markets by Category
```json
{
  "tool": "web_fetch",
  "url": "https://gamma-api.polymarket.com/events?active=true&tag=crypto",
  "extract_mode": "raw"
}
```

### Get Market Resolution Data
```json
{
  "tool": "web_fetch",
  "url": "https://gamma-api.polymarket.com/markets?id={{market_id}}",
  "extract_mode": "raw"
}
```

---

## Important Notes

1. **Token IDs**: The `conditionId` from market data is used as `token_id` in CLOB API calls
2. **Rate Limits**: Be mindful of API rate limits; cache responses when possible
3. **Price Updates**: Prices change frequently; always fetch fresh data for trading decisions
4. **Binary Markets**: Most markets are binary (YES/NO) - the YES and NO token prices should sum to approximately 1.00
5. **Resolution**: Markets resolve based on real-world outcomes; check the `resolution_source` field for details

---

## Error Handling

If an API call fails:
1. Check the URL format and parameters
2. Verify the market/event exists and is active
3. Try with a different token_id if the market has multiple outcome tokens
4. Check if the market has been resolved (closed markets may have limited data)

Common HTTP status codes:
- `200`: Success
- `400`: Bad request (check parameters)
- `404`: Market/event not found
- `429`: Rate limited (wait and retry)
- `500`: Server error (retry later)

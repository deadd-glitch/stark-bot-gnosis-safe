---
name: swap
description: "Swap ERC20 tokens on Base using 0x DEX aggregator via quoter.defirelay.com"
version: 2.0.0
author: starkbot
homepage: https://0x.org
metadata: {"requires_auth": false, "clawdbot":{"emoji":"🔄"}}
tags: [crypto, defi, swap, dex, base, trading, 0x]
---

# Token Swap Integration (0x via DeFi Relay)

Swap ERC20 tokens on Base using the 0x DEX aggregator. Uses `quoter.defirelay.com` with x402 payment protocol.

## Tools Used

| Tool | Purpose |
|------|---------|
| `local_burner_wallet` | Get wallet address |
| `x402_fetch` | Get swap quote from 0x |
| `web3_function_call` | ERC20 approve (uses ABI, no hex needed!) |
| `web3_tx` | Execute swap (uses pre-encoded data from 0x) |

## Common Token Addresses (Base)

| Token | Address |
|-------|---------|
| WETH | `0x4200000000000000000000000000000000000006` |
| USDC | `0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913` |
| BNKR | `0x22aF33FE49fD1Fa80c7149773dDe5890D3c76F3b` |
| cbBTC | `0xcbB7C0000aB88B473b1f5aFd9ef808440eed33Bf` |
| DAI | `0x50c5725949A6F0c72E6C4a641F24049A917DB0Cb` |
| USDbC | `0xd9aAEc86B65D86f6A7B5B1b0c42FFA531710b6CA` |

**Native ETH**: Use `0xEeeeeEeeeEeEeeEeEeEeeEEEeeeeEeeeeeeeEEeE` as sellToken when selling ETH.

---

## How to Swap Tokens

### Step 1: Get Wallet Address

Use `local_burner_wallet`:
```json
{"action": "address"}
```

### Step 2: Get a Quote

Use `x402_fetch` to call the quoter API:

```json
{
  "url": "https://quoter.defirelay.com/swap/allowance-holder/quote?chainId=8453&sellToken=<SELL_TOKEN>&buyToken=<BUY_TOKEN>&sellAmount=<SELL_AMOUNT>&taker=<WALLET_ADDRESS>",
  "jq_filter": "{to: .transaction.to, data: .transaction.data, value: .transaction.value, gas: .transaction.gas, buyAmount: .buyAmount, issues: .issues}"
}
```

The response gives you everything needed:
- `to` - Contract address to call
- `data` - Pre-encoded calldata (DO NOT MODIFY!)
- `value` - ETH value to send
- `gas` - Gas estimate
- `buyAmount` - Expected output tokens
- `issues` - Approval requirements (if any)

### Step 3: Check for Allowance Issues

The quote response includes an `issues` field:

```json
{
  "issues": {
    "allowance": {
      "spender": "0x0000000000001fF3684f28c67538d4D072C22734",
      "actual": "0",
      "expected": "1000000"
    }
  }
}
```

If `issues.allowance` exists and `actual < expected`, approve the token first.

### Step 4: Approve Token (if needed)

**Use `web3_function_call` - NO HEX ENCODING NEEDED!**

```json
{
  "abi": "erc20",
  "contract": "<SELL_TOKEN_ADDRESS>",
  "function": "approve",
  "params": [
    "0x0000000000001fF3684f28c67538d4D072C22734",
    "115792089237316195423570985008687907853269984665640564039457584007913129639935"
  ],
  "network": "base"
}
```

The `params` array is:
1. Spender address (0x AllowanceHolder contract)
2. Amount to approve (max uint256 for unlimited)

### Step 5: Execute the Swap

**Use `web3_tx` with the EXACT values from the quote response:**

```json
{
  "to": "<to from quote>",
  "data": "<data from quote - PASS THROUGH EXACTLY!>",
  "value": "<value from quote>",
  "gas_limit": "<gas from quote>",
  "network": "base"
}
```

**IMPORTANT:** Pass the `data` field from the quote response EXACTLY as received. Do not modify, parse, or reconstruct it!

---

## Complete Example: Swap 0.01 ETH for USDC

### 1. Get wallet address

```json
// local_burner_wallet
{"action": "address"}
```
Response: `0xYourWalletAddress`

### 2. Get quote

```json
// x402_fetch
{
  "url": "https://quoter.defirelay.com/swap/allowance-holder/quote?chainId=8453&sellToken=0xEeeeeEeeeEeEeeEeEeEeeEEEeeeeEeeeeeeeEEeE&buyToken=0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913&sellAmount=10000000000000000&taker=0xYourWalletAddress",
  "jq_filter": "{to: .transaction.to, data: .transaction.data, value: .transaction.value, gas: .transaction.gas, buyAmount: .buyAmount, issues: .issues}"
}
```

Response:
```json
{
  "to": "0x0000000000001fF3684f28c67538d4D072C22734",
  "data": "0x1fff991f000000000000000000000000833589...<long hex>",
  "value": "10000000000000000",
  "gas": "200000",
  "buyAmount": "25123456",
  "issues": null
}
```

### 3. Execute swap (no approval needed for ETH)

```json
// web3_tx
{
  "to": "0x0000000000001fF3684f28c67538d4D072C22734",
  "data": "0x1fff991f000000000000000000000000833589...<exact data from quote>",
  "value": "10000000000000000",
  "gas_limit": "200000",
  "network": "base"
}
```

---

## Example: Swap USDC for ETH (requires approval)

### 1. Get wallet address

```json
// local_burner_wallet
{"action": "address"}
```

### 2. Get quote

```json
// x402_fetch
{
  "url": "https://quoter.defirelay.com/swap/allowance-holder/quote?chainId=8453&sellToken=0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913&buyToken=0xEeeeeEeeeEeEeeEeEeEeeEEEeeeeEeeeeeeeEEeE&sellAmount=10000000&taker=0xYourWalletAddress",
  "jq_filter": "{to: .transaction.to, data: .transaction.data, value: .transaction.value, gas: .transaction.gas, buyAmount: .buyAmount, issues: .issues}"
}
```

Response shows allowance issue:
```json
{
  "to": "0x0000000000001fF3684f28c67538d4D072C22734",
  "data": "0x...",
  "value": "0",
  "gas": "250000",
  "buyAmount": "3980000000000000",
  "issues": {
    "allowance": {
      "spender": "0x0000000000001fF3684f28c67538d4D072C22734",
      "actual": "0",
      "expected": "10000000"
    }
  }
}
```

### 3. Approve USDC (using web3_function_call - no hex!)

```json
// web3_function_call
{
  "abi": "erc20",
  "contract": "0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913",
  "function": "approve",
  "params": [
    "0x0000000000001fF3684f28c67538d4D072C22734",
    "115792089237316195423570985008687907853269984665640564039457584007913129639935"
  ],
  "network": "base"
}
```

### 4. Execute swap

```json
// web3_tx
{
  "to": "0x0000000000001fF3684f28c67538d4D072C22734",
  "data": "<data from quote - EXACT!>",
  "value": "0",
  "gas_limit": "250000",
  "network": "base"
}
```

---

## Amount Conversion Reference

| Token | Decimals | 1 Token in Wei |
|-------|----------|----------------|
| ETH/WETH | 18 | 1000000000000000000 |
| USDC | 6 | 1000000 |
| BNKR | 18 | 1000000000000000000 |
| cbBTC | 8 | 100000000 |

**Quick conversions:**
- 0.01 ETH = `10000000000000000` wei
- 0.1 ETH = `100000000000000000` wei
- 1 ETH = `1000000000000000000` wei
- 1 USDC = `1000000` (6 decimals)
- 10 USDC = `10000000`
- 100 USDC = `100000000`

---

## Check Token Balance

Use `web3_function_call` with `call_only: true`:

```json
{
  "abi": "erc20",
  "contract": "0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913",
  "function": "balanceOf",
  "params": ["<wallet_address>"],
  "network": "base",
  "call_only": true
}
```

---

## Error Handling

**Common errors:**

1. **"Insufficient balance"** - Check token balances
2. **"Insufficient allowance"** - Need to approve token first (Step 3-4)
3. **"Transaction reverted"** - Slippage too high, try fresh quote
4. **"Gas estimation failed"** - Swap would fail, check amounts

**If swap fails:**
1. Check token balances with `web3_function_call` (balanceOf)
2. Get a fresh quote (quotes expire quickly!)
3. Ensure approval was successful for ERC20 sells
4. Verify you're passing the quote `data` field exactly

---

## Security Notes

1. **Always verify addresses** - Double-check token addresses
2. **Start small** - Test with small amounts first
3. **Quotes expire** - Execute promptly after getting a quote
4. **Don't modify calldata** - Pass the `data` from quote exactly as-is

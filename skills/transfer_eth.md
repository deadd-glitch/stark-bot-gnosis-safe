---
name: transfer_eth
description: "Transfer (Send) native ETH on Base/Ethereum using the burner wallet"
version: 2.3.0
author: starkbot
homepage: https://basescan.org
metadata: {"requires_auth": false, "clawdbot":{"emoji":"💸"}}
tags: [crypto, transfer, send, eth, base, wallet]
requires_tools: [set_address, to_raw_amount, send_eth, list_queued_web3_tx, broadcast_web3_tx, x402_rpc, select_web3_network]
---

# ETH Transfer/Send Skill

Transfer or Send native ETH from the burner wallet to any address.

> **IMPORTANT: This skill uses the REGISTER PATTERN to prevent hallucination of transaction data.**
>
> - Use `set_address` to set `send_to` (recipient address) — validates address format
> - Use `to_raw_amount` with `decimals: 18` to set `amount_raw` (wei value)
> - The `send_eth` tool reads from these registers - you NEVER pass raw tx params directly

## Step 0: Network Selection (If Specified)

**Before ANY transfer operation, check if the user specified a network in their query.**

If the user mentions a specific network (e.g., "on polygon", "on mainnet", "on base"), you MUST call `select_web3_network` FIRST:

```json
{"tool": "select_web3_network", "network": "<network_from_query>"}
```

**Examples of network detection:**
- "send 0.1 ETH **on polygon**" -> `{"tool": "select_web3_network", "network": "polygon"}`
- "transfer 0.01 ETH **on mainnet**" -> `{"tool": "select_web3_network", "network": "mainnet"}`
- "send ETH **on arbitrum**" -> `{"tool": "select_web3_network", "network": "arbitrum"}`

**If no network is specified**, proceed with the current/default network (usually base).

---

## Tools Used

| Tool | Purpose |
|------|---------|
| `x402_rpc` | Get gas price and ETH balance (get_balance preset) |
| `set_address` | Set the recipient address (`send_to` register) — validates format |
| `to_raw_amount` | Convert human ETH amount to wei (`amount_raw` register) |
| `send_eth` | Execute native ETH transfers (reads from registers) |

**Note:** `wallet_address` is an intrinsic register - always available automatically.

---

## Required Tool Flow

**ALWAYS follow this sequence for ETH transfers:**

0. `select_web3_network` -> **If user specified a network** (e.g., "on polygon")
1. `set_address` -> Set `send_to` (recipient address)
2. `to_raw_amount` -> Convert human amount to wei (sets `amount_raw`)
3. `send_eth` -> Execute the transfer (reads from registers)

---

## Step 1: Set the recipient address

```json
{"tool": "set_address", "register": "send_to", "address": "0x1234567890abcdef1234567890abcdef12345678"}
```

---

## Step 2: Convert amount to wei

Use `to_raw_amount` with `decimals: 18` (ETH always has 18 decimals):

```tool:to_raw_amount
amount: "0.01"
decimals: 18
cache_as: amount_raw
```

This converts 0.01 ETH -> "10000000000000000" wei

---

## Step 3: Execute the transfer

```tool:send_eth
network: base
```

The tool reads `send_to` and `amount_raw` from registers automatically.
Gas is auto-estimated (21000 for simple ETH transfers).

---

## Step 4: Verify and Broadcast

Verify the queued transaction:
```tool:list_queued_web3_tx
status: pending
limit: 1
```

Broadcast when ready:
```tool:broadcast_web3_tx
```

---

## Complete Example: Send 0.01 ETH

### 1. Set recipient address

```json
{"tool": "set_address", "register": "send_to", "address": "0x1234567890abcdef1234567890abcdef12345678"}
```

### 2. Convert amount to wei

```tool:to_raw_amount
amount: "0.01"
decimals: 18
cache_as: amount_raw
```

### 3. Execute transfer

```tool:send_eth
network: base
```

### 4. Verify and broadcast

```tool:list_queued_web3_tx
status: pending
limit: 1
```

```tool:broadcast_web3_tx
```

---

## Check ETH Balance

```tool:x402_rpc
preset: get_balance
network: base
```

The result is hex wei - convert to ETH by dividing by 10^18.

---

## Amount Reference

| Human Amount | Wei Value (from to_raw_amount) |
|--------------|--------------------------------|
| 0.0001 ETH | `100000000000000` |
| 0.001 ETH | `1000000000000000` |
| 0.01 ETH | `10000000000000000` |
| 0.1 ETH | `100000000000000000` |
| 1 ETH | `1000000000000000000` |

---

## CRITICAL RULES

### Always use to_raw_amount for amounts!
**This prevents incorrect amounts from being sent.** The `to_raw_amount` tool:
1. Validates the human amount is a valid number
2. Correctly multiplies by 10^18 for ETH
3. Stores the result in `amount_raw` register

---

## Pre-Transfer Checklist

Before executing a transfer:

1. **Verify recipient address** - Double-check the address is correct
2. **Check balance** - Use `x402_rpc` (get_balance) for ETH
3. **Use to_raw_amount** - Never manually calculate wei values
4. **Network** - Confirm you're on the right network (base vs mainnet)

---

## Error Handling

| Error | Cause | Solution |
|-------|-------|----------|
| "Insufficient funds" | Not enough ETH for gas + value | Add ETH to wallet |
| "Gas estimation failed" | Invalid recipient or params | Verify addresses |
| "Transaction reverted" | Should not happen for simple ETH transfer | Check recipient is not a contract that rejects ETH |
| "Register 'send_to' not found" | Missing recipient | Use set_address first |
| "Register 'amount_raw' not found" | Missing amount | Use to_raw_amount first |

---

## Security Notes

1. **Register pattern prevents hallucination** - tx data flows through validated registers
2. **set_address validates addresses** - rejects invalid formats and zero address
3. **to_raw_amount validates amounts** - prevents incorrect decimal conversions
4. **Always double-check addresses** - Transactions cannot be reversed
5. **Start with small test amounts** - Verify the flow works first
6. **Gas costs** - ETH needed for gas (21000 gas for simple transfer)

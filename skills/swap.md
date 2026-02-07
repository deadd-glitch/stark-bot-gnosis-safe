---
name: swap
description: "Swap ERC20 tokens on Base using 0x DEX aggregator via quoter.defirelay.com"
version: 7.2.0
author: starkbot
homepage: https://0x.org
metadata: {"requires_auth": false, "clawdbot":{"emoji":"🔄"}}
tags: [crypto, defi, swap, dex, base, trading, 0x]
requires_tools: [token_lookup, to_raw_amount, decode_calldata, web3_preset_function_call, x402_fetch, x402_rpc, list_queued_web3_tx, broadcast_web3_tx, select_web3_network, add_task]
---

# Token Swap Skill

This skill has TWO phases, split into separate tasks so nothing gets skipped.

**Phase 1 (this task):** Look up tokens, check allowance, split remaining work into tasks.
**Phase 2 (next task, if needed):** Approve sell token for Permit2.
**Phase 3 (next task):** Execute the actual swap.

---

## Phase 1: Prepare tokens and plan tasks

### 1a. Select network (if user specified one)

```json
{"tool": "select_web3_network", "network": "<network>"}
```

### 1b. Look up SELL token

```json
{"tool": "token_lookup", "symbol": "<SELL_TOKEN>", "cache_as": "sell_token"}
```

**If selling ETH:** use WETH as the sell token instead:
1. Lookup WETH: `{"tool": "token_lookup", "symbol": "WETH", "cache_as": "sell_token"}`
2. Check WETH balance: `{"tool": "web3_preset_function_call", "preset": "weth_balance", "network": "<network>", "call_only": true}`
3. Check ETH balance: `{"tool": "x402_rpc", "preset": "get_balance", "network": "<network>"}`
4. If WETH insufficient, wrap:
   - `{"tool": "to_raw_amount", "amount": "<human_amount>", "decimals": 18, "cache_as": "wrap_amount"}`
   - `{"tool": "web3_preset_function_call", "preset": "weth_deposit", "network": "<network>"}`
   - Broadcast the wrap tx and wait for confirmation

### 1c. Look up BUY token

```json
{"tool": "token_lookup", "symbol": "<BUY_TOKEN>", "cache_as": "buy_token"}
```

### 1d. Check Permit2 allowance

```json
{"tool": "web3_preset_function_call", "preset": "erc20_allowance_permit2", "network": "<network>", "call_only": true}
```

### 1e. Create tasks and finish Phase 1

Now split the remaining work into separate tasks. The order you call `add_task` matters — **add the swap task first, then add the approval task** (which pushes it to the front, before the swap).

**If allowance is 0 or less than sell amount (approval needed):**

```json
{"tool": "add_task", "description": "Execute swap: fetch quote, decode with cache_as swap, call swap_execute preset, broadcast, and verify the swap transaction is CONFIRMED before reporting success. Use the swap skill Phase 3 instructions.", "position": "front"}
```
```json
{"tool": "add_task", "description": "Approve sell token for Permit2: call erc20_approve_permit2 preset, broadcast and wait for confirmation. This is ONLY the approval — the swap is a separate task.", "position": "front"}
```
```json
{"tool": "task_fully_completed", "summary": "Token lookups done. Approval needed — created approval and swap tasks."}
```

**If allowance is sufficient (no approval needed):**

```json
{"tool": "add_task", "description": "Execute swap: fetch quote, decode with cache_as swap, call swap_execute preset, broadcast, and verify the swap transaction is CONFIRMED before reporting success. Use the swap skill Phase 3 instructions.", "position": "front"}
```
```json
{"tool": "task_fully_completed", "summary": "Token lookups done. Allowance sufficient — created swap task."}
```

---

## Phase 2: Approve sell token (only if approval was needed)

This task runs only if an approval task was created in Phase 1.

```json
{"tool": "web3_preset_function_call", "preset": "erc20_approve_permit2", "network": "<network>"}
```

Broadcast and wait for confirmation:
```json
{"tool": "broadcast_web3_tx", "uuid": "<uuid_from_approve>"}
```

After the approval is confirmed:
```json
{"tool": "task_fully_completed", "summary": "Sell token approved for Permit2. Ready to execute swap."}
```

**The approval is NOT the swap. Do NOT report success to the user yet.**

---

## Phase 3: Execute the swap

This is the actual swap. Follow these steps exactly.

### 3a. Convert sell amount to wei

```json
{"tool": "to_raw_amount", "amount": "<human_amount>", "decimals_register": "sell_token_decimals", "cache_as": "sell_amount"}
```

### 3b. Fetch swap quote

```json
{"tool": "x402_fetch", "preset": "swap_quote", "cache_as": "swap_quote", "network": "<network>"}
```

If this fails after retries, STOP and tell the user.

### 3c. Decode the quote

**Use `cache_as: "swap"` exactly.** This sets `swap_param_0`–`swap_param_4`, `swap_contract`, `swap_value`.

```json
{"tool": "decode_calldata", "abi": "0x_settler", "calldata_register": "swap_quote", "cache_as": "swap"}
```

### 3d. Execute the swap transaction

```json
{"tool": "web3_preset_function_call", "preset": "swap_execute", "network": "<network>"}
```

### 3e. Broadcast the swap transaction

```json
{"tool": "broadcast_web3_tx", "uuid": "<uuid_from_3d>"}
```

### 3f. VERIFY the result

Read the output of `broadcast_web3_tx`:

- **"TRANSACTION CONFIRMED"** → The swap succeeded. Report success with tx hash and explorer link.
- **"TRANSACTION REVERTED"** → The swap FAILED. Tell the user. Do NOT call `task_fully_completed`.
- **"confirmation timeout"** → Tell the user to check the explorer link.

**Only call `task_fully_completed` if the swap broadcast returned CONFIRMED.**

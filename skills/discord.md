---
name: discord
description: "Control Discord: send messages, react, post stickers/emojis, run polls, manage threads/pins, fetch permissions/member/role/channel info, handle moderation."
version: 1.0.0
author: starkbot
metadata: {"clawdbot":{"emoji":"🎮"}}
tags: [discord, social, messaging, communication, social-media]
requires_tools: [discord, discord_lookup, agent_send]
---

# Discord Actions

## Overview

Use `discord` to manage messages, reactions, threads, polls, and moderation. You can disable groups via `discord.actions.*` (defaults to enabled, except roles/moderation). The tool uses the bot token configured for Clawdbot.

## Default Channel

**If no channel is specified, default to the "general" channel.** Use `discord_lookup` with `action: search_channels` and `query: "general"` to find it if you don't have the channel ID.

## Inputs to collect

- For reactions: `channelId`, `messageId`, and an `emoji`.
- For stickers/polls/sendMessage: a `to` target (`channel:<id>` or `user:<id>`). Optional `content` text. **If no channel specified, use "general".**
- Polls also need a `question` plus 2–10 `answers`.
- For media: `mediaUrl` with `file:///path` for local files or `https://...` for remote.
- For emoji uploads: `guildId`, `name`, `mediaUrl`, optional `roleIds` (limit 256KB, PNG/JPG/GIF).
- For sticker uploads: `guildId`, `name`, `description`, `tags`, `mediaUrl` (limit 512KB, PNG/APNG/Lottie JSON).

Message context lines include `discord message id` and `channel` fields you can reuse directly.

**Note:** `sendMessage` uses `to: "channel:<id>"` format, not `channelId`. Other actions like `react`, `readMessages`, `editMessage` use `channelId` directly.

## Actions

### React to a message

```tool:discord
action: react
channelId: "123"
messageId: "456"
emoji: "✅"
```

### List reactions + users

```tool:discord
action: reactions
channelId: "123"
messageId: "456"
limit: 100
```

### Send a sticker

```tool:discord
action: sticker
to: "channel:123"
stickerIds: ["9876543210"]
content: "Nice work!"
```

- Up to 3 sticker IDs per message.
- `to` can be `user:<id>` for DMs.

### Upload a custom emoji

```tool:discord
action: emojiUpload
guildId: "999"
name: party_blob
mediaUrl: "file:///tmp/party.png"
roleIds: ["222"]
```

- Emoji images must be PNG/JPG/GIF and <= 256KB.
- `roleIds` is optional; omit to make the emoji available to everyone.

### Upload a sticker

```tool:discord
action: stickerUpload
guildId: "999"
name: clawdbot_wave
description: "Clawdbot waving hello"
tags: "👋"
mediaUrl: "file:///tmp/wave.png"
```

- Stickers require `name`, `description`, and `tags`.
- Uploads must be PNG/APNG/Lottie JSON and <= 512KB.

### Create a poll

```tool:discord
action: poll
to: "channel:123"
question: "Lunch?"
answers: ["Pizza", "Sushi", "Salad"]
allowMultiselect: false
durationHours: 24
content: "Vote now"
```

- `durationHours` defaults to 24; max 32 days (768 hours).

### Check bot permissions for a channel

```tool:discord
action: permissions
channelId: "123"
```

## Ideas to try

- React with ✅/⚠️ to mark status updates.
- Post a quick poll for release decisions or meeting times.
- Send celebratory stickers after successful deploys.
- Upload new emojis/stickers for release moments.
- Run weekly "priority check" polls in team channels.
- DM stickers as acknowledgements when a user's request is completed.

## Finding Servers and Channels by Name

Use `discord_lookup` to find server/channel IDs when you only know the name:

### List all servers the bot is in

```tool:discord_lookup
action: list_servers
```

### Search for a server by name

```tool:discord_lookup
action: search_servers
query: "starkbot"
```

### List channels in a server

```tool:discord_lookup
action: list_channels
server_id: "123456789"
```

### Search for a channel by name

```tool:discord_lookup
action: search_channels
server_id: "123456789"
query: "general"
```

### Quick send with agent_send

For simple messages without the full discord tool:

```tool:agent_send
channel: "123456789012345678"
message: "Hello!"
platform: discord
```

## Action gating

Use `discord.actions.*` to disable action groups:
- `reactions` (react + reactions list + emojiList)
- `stickers`, `polls`, `permissions`, `messages`, `threads`, `pins`, `search`
- `emojiUploads`, `stickerUploads`
- `memberInfo`, `roleInfo`, `channelInfo`, `voiceStatus`, `events`
- `roles` (role add/remove, default `false`)
- `moderation` (timeout/kick/ban, default `false`)

### Read recent messages

```tool:discord
action: readMessages
channelId: "123"
limit: 20
```

### Send/edit/delete a message

**If the user doesn't specify a channel, default to "general".** Look up the general channel ID first using `discord_lookup` if needed.

```tool:discord
action: sendMessage
to: "channel:123"
content: "Hello from Clawdbot"
```

**With media attachment:**

```tool:discord
action: sendMessage
to: "channel:123"
content: "Check out this audio!"
mediaUrl: "file:///tmp/audio.mp3"
```

- `to` uses format `channel:<id>` or `user:<id>` for DMs (not `channelId`!)
- `mediaUrl` supports local files (`file:///path/to/file`) and remote URLs (`https://...`)
- Optional `replyTo` with a message ID to reply to a specific message

```tool:discord
action: editMessage
channelId: "123"
messageId: "456"
content: "Fixed typo"
```

```tool:discord
action: deleteMessage
channelId: "123"
messageId: "456"
```

### Threads

```tool:discord
action: threadCreate
channelId: "123"
name: "Bug triage"
messageId: "456"
```

```tool:discord
action: threadList
guildId: "999"
```

```tool:discord
action: threadReply
channelId: "777"
content: "Replying in thread"
```

### Pins

```tool:discord
action: pinMessage
channelId: "123"
messageId: "456"
```

```tool:discord
action: listPins
channelId: "123"
```

### Search messages

```tool:discord
action: searchMessages
guildId: "999"
content: "release notes"
channelIds: ["123", "456"]
limit: 10
```

### Member + role info

```tool:discord
action: memberInfo
guildId: "999"
userId: "111"
```

```tool:discord
action: roleInfo
guildId: "999"
```

### List available custom emojis

```tool:discord
action: emojiList
guildId: "999"
```

### Role changes (disabled by default)

```tool:discord
action: roleAdd
guildId: "999"
userId: "111"
roleId: "222"
```

### Channel info

```tool:discord
action: channelInfo
channelId: "123"
```

```tool:discord
action: channelList
guildId: "999"
```

### Voice status

```tool:discord
action: voiceStatus
guildId: "999"
userId: "111"
```

### Scheduled events

```tool:discord
action: eventList
guildId: "999"
```

### Moderation (disabled by default)

```tool:discord
action: timeout
guildId: "999"
userId: "111"
durationMinutes: 10
```

## Discord Writing Style Guide

**Keep it conversational!** Discord is a chat platform, not documentation.

### Do
- Short, punchy messages (1-3 sentences ideal)
- Multiple quick replies > one wall of text
- Use emoji for tone/emphasis 🦞
- Lowercase casual style is fine
- Break up info into digestible chunks
- Match the energy of the conversation

### Don't
- No markdown tables (Discord renders them as ugly raw `| text |`)
- No `## Headers` for casual chat (use **bold** or CAPS for emphasis)
- Avoid multi-paragraph essays
- Don't over-explain simple things
- Skip the "I'd be happy to help!" fluff

### Formatting that works
- **bold** for emphasis
- `code` for technical terms
- Lists for multiple items
- > quotes for referencing
- Wrap multiple links in `<>` to suppress embeds

### Example transformations

❌ Bad:
```
I'd be happy to help with that! Here's a comprehensive overview of the versioning strategies available:

## Semantic Versioning
Semver uses MAJOR.MINOR.PATCH format where...

## Calendar Versioning
CalVer uses date-based versions like...
```

✅ Good:
```
versioning options: semver (1.2.3), calver (2026.01.04), or yolo (`latest` forever). what fits your release cadence?
```

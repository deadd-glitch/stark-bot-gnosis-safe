use crate::channels::dispatcher::MessageDispatcher;
use crate::channels::types::{ChannelType, NormalizedMessage};
use crate::gateway::events::EventBroadcaster;
use crate::gateway::protocol::GatewayEvent;
use crate::models::Channel;
use serenity::all::{
    Client, Context, EventHandler, GatewayIntents, Message, Ready,
};
use std::sync::Arc;
use tokio::sync::oneshot;

struct DiscordHandler {
    channel_id: i64,
    dispatcher: Arc<MessageDispatcher>,
}

#[serenity::async_trait]
impl EventHandler for DiscordHandler {
    async fn message(&self, ctx: Context, msg: Message) {
        // Ignore messages from bots (including ourselves)
        if msg.author.bot {
            return;
        }

        let text = msg.content.clone();
        if text.is_empty() {
            return;
        }

        let user_id = msg.author.id.to_string();
        // Discord moved away from discriminators, so just use the username
        // If discriminator exists and is non-zero, include it for backwards compatibility
        let user_name = match msg.author.discriminator {
            Some(disc) => format!("{}#{}", msg.author.name, disc),
            None => msg.author.name.clone(),
        };

        log::info!(
            "Discord: Message from {} ({}): {}",
            user_name,
            user_id,
            if text.len() > 50 { &text[..50] } else { &text }
        );

        let normalized = NormalizedMessage {
            channel_id: self.channel_id,
            channel_type: ChannelType::Discord.to_string(),
            chat_id: msg.channel_id.to_string(),
            user_id,
            user_name: user_name.clone(),
            text,
            message_id: Some(msg.id.to_string()),
        };

        // Dispatch to AI
        log::info!("Discord: Dispatching message to AI for user {}", user_name);
        let result = self.dispatcher.dispatch(normalized).await;
        log::info!("Discord: Dispatch complete, error={:?}", result.error);

        // Send response
        if result.error.is_none() && !result.response.is_empty() {
            // Discord has a 2000 character limit per message
            let response = &result.response;
            let chunks = split_message(response, 2000);

            for chunk in chunks {
                if let Err(e) = msg.channel_id.say(&ctx.http, &chunk).await {
                    log::error!("Failed to send Discord message: {}", e);
                }
            }
        } else if let Some(error) = result.error {
            let error_msg = format!("Sorry, I encountered an error: {}", error);
            let _ = msg.channel_id.say(&ctx.http, &error_msg).await;
        }
    }

    async fn ready(&self, _ctx: Context, ready: Ready) {
        log::info!("Discord: Bot connected as {}", ready.user.name);
    }
}

/// Split a message into chunks respecting Discord's character limit
fn split_message(text: &str, max_len: usize) -> Vec<String> {
    if text.len() <= max_len {
        return vec![text.to_string()];
    }

    let mut chunks = Vec::new();
    let mut current = String::new();

    for line in text.lines() {
        if current.len() + line.len() + 1 > max_len {
            if !current.is_empty() {
                chunks.push(current);
                current = String::new();
            }
            // If single line is too long, split it
            if line.len() > max_len {
                let mut remaining = line;
                while remaining.len() > max_len {
                    chunks.push(remaining[..max_len].to_string());
                    remaining = &remaining[max_len..];
                }
                if !remaining.is_empty() {
                    current = remaining.to_string();
                }
            } else {
                current = line.to_string();
            }
        } else {
            if !current.is_empty() {
                current.push('\n');
            }
            current.push_str(line);
        }
    }

    if !current.is_empty() {
        chunks.push(current);
    }

    chunks
}

/// Start a Discord bot listener
pub async fn start_discord_listener(
    channel: Channel,
    dispatcher: Arc<MessageDispatcher>,
    broadcaster: Arc<EventBroadcaster>,
    mut shutdown_rx: oneshot::Receiver<()>,
) -> Result<(), String> {
    let channel_id = channel.id;
    let channel_name = channel.name.clone();
    let bot_token = channel.bot_token.clone();

    log::info!("Starting Discord listener for channel: {}", channel_name);
    log::info!("Discord: Token length = {}", bot_token.len());

    // Set up intents - we need message content to read messages
    let intents = GatewayIntents::GUILD_MESSAGES
        | GatewayIntents::DIRECT_MESSAGES
        | GatewayIntents::MESSAGE_CONTENT;

    let handler = DiscordHandler {
        channel_id,
        dispatcher,
    };

    // Create client
    let mut client = Client::builder(&bot_token, intents)
        .event_handler(handler)
        .await
        .map_err(|e| format!("Failed to create Discord client: {}", e))?;

    log::info!("Discord: Client created successfully");

    // Emit started event
    broadcaster.broadcast(GatewayEvent::channel_started(
        channel_id,
        ChannelType::Discord.as_str(),
        &channel_name,
    ));

    // Get shard manager for shutdown
    let shard_manager = client.shard_manager.clone();

    // Run with shutdown signal
    tokio::select! {
        _ = &mut shutdown_rx => {
            log::info!("Discord listener {} received shutdown signal", channel_name);
            shard_manager.shutdown_all().await;
        }
        result = client.start() => {
            match result {
                Ok(()) => log::info!("Discord listener {} stopped", channel_name),
                Err(e) => {
                    let error = format!("Discord client error: {}", e);
                    log::error!("{}", error);
                    broadcaster.broadcast(GatewayEvent::channel_stopped(
                        channel_id,
                        ChannelType::Discord.as_str(),
                        &channel_name,
                    ));
                    return Err(error);
                }
            }
        }
    }

    // Emit stopped event
    broadcaster.broadcast(GatewayEvent::channel_stopped(
        channel_id,
        ChannelType::Discord.as_str(),
        &channel_name,
    ));

    Ok(())
}

use crate::channels::dispatcher::MessageDispatcher;
use crate::channels::types::{ChannelType, NormalizedMessage};
use crate::gateway::events::EventBroadcaster;
use crate::gateway::protocol::GatewayEvent;
use crate::models::Channel;
use std::sync::Arc;
use teloxide::prelude::*;
use teloxide::requests::Requester;
use tokio::sync::oneshot;

/// Start a Telegram bot listener
pub async fn start_telegram_listener(
    channel: Channel,
    dispatcher: Arc<MessageDispatcher>,
    broadcaster: Arc<EventBroadcaster>,
    shutdown_rx: oneshot::Receiver<()>,
) -> Result<(), String> {
    let channel_id = channel.id;
    let channel_name = channel.name.clone();
    let bot_token = channel.bot_token.clone();

    log::info!("Starting Telegram listener for channel: {}", channel_name);
    log::info!("Telegram: Token length = {}", bot_token.len());

    // Create the bot
    let bot = Bot::new(&bot_token);

    // Validate token by calling getMe
    log::info!("Telegram: Validating bot token...");
    match bot.get_me().await {
        Ok(me) => {
            log::info!(
                "Telegram: Bot validated - username: @{}, id: {}",
                me.username(),
                me.id
            );
        }
        Err(e) => {
            let error = format!("Invalid Telegram bot token: {}", e);
            log::error!("Telegram: {}", error);
            return Err(error);
        }
    }

    // Emit started event
    broadcaster.broadcast(GatewayEvent::channel_started(
        channel_id,
        ChannelType::Telegram.as_str(),
        &channel_name,
    ));

    // Create message handler
    let handler = Update::filter_message().endpoint(
        move |bot: Bot, msg: teloxide::types::Message, dispatcher: Arc<MessageDispatcher>| {
            let channel_id = channel_id;
            async move {
                log::info!("Telegram: Received update from chat {}", msg.chat.id);

                // Only handle text messages
                if let Some(text) = msg.text() {
                    let user = msg.from();
                    let user_id = user.map(|u| u.id.to_string()).unwrap_or_default();
                    let user_name = user
                        .map(|u| {
                            u.username
                                .clone()
                                .unwrap_or_else(|| u.first_name.clone())
                        })
                        .unwrap_or_else(|| "Unknown".to_string());

                    log::info!(
                        "Telegram: Message from {} ({}): {}",
                        user_name,
                        user_id,
                        if text.len() > 50 { &text[..50] } else { text }
                    );

                    let normalized = NormalizedMessage {
                        channel_id,
                        channel_type: ChannelType::Telegram.to_string(),
                        chat_id: msg.chat.id.to_string(),
                        user_id,
                        user_name: user_name.clone(),
                        text: text.to_string(),
                        message_id: Some(msg.id.to_string()),
                    };

                    // Dispatch to AI
                    log::info!("Telegram: Dispatching message to AI for user {}", user_name);
                    let result = dispatcher.dispatch(normalized).await;
                    log::info!("Telegram: Dispatch complete, error={:?}", result.error);

                    // Send response
                    if result.error.is_none() && !result.response.is_empty() {
                        if let Err(e) = bot
                            .send_message(msg.chat.id, &result.response)
                            .reply_to_message_id(msg.id)
                            .await
                        {
                            log::error!("Failed to send Telegram message: {}", e);
                        }
                    } else if let Some(error) = result.error {
                        // Send error message
                        let error_msg = format!("Sorry, I encountered an error: {}", error);
                        let _ = bot
                            .send_message(msg.chat.id, &error_msg)
                            .reply_to_message_id(msg.id)
                            .await;
                    }
                }

                Ok::<(), Box<dyn std::error::Error + Send + Sync>>(())
            }
        },
    );

    // Create dispatcher
    let mut tg_dispatcher = Dispatcher::builder(bot, handler)
        .dependencies(dptree::deps![dispatcher])
        .enable_ctrlc_handler()
        .build();

    // Run with shutdown signal
    tokio::select! {
        _ = shutdown_rx => {
            log::info!("Telegram listener {} received shutdown signal", channel_name);
        }
        _ = tg_dispatcher.dispatch() => {
            log::info!("Telegram listener {} stopped", channel_name);
        }
    }

    // Emit stopped event
    broadcaster.broadcast(GatewayEvent::channel_stopped(
        channel_id,
        ChannelType::Telegram.as_str(),
        &channel_name,
    ));

    Ok(())
}

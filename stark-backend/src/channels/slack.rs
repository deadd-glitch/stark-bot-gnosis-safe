use crate::channels::dispatcher::MessageDispatcher;
use crate::channels::types::ChannelType;
use crate::gateway::events::EventBroadcaster;
use crate::gateway::protocol::GatewayEvent;
use crate::models::Channel;
use slack_morphism::prelude::*;
use std::sync::Arc;
use tokio::sync::oneshot;

/// Start a Slack bot listener using Socket Mode
///
/// Note: Slack Socket Mode requires a complex setup with event subscriptions.
/// This implementation provides the basic framework for receiving messages.
pub async fn start_slack_listener(
    channel: Channel,
    _dispatcher: Arc<MessageDispatcher>,
    broadcaster: Arc<EventBroadcaster>,
    shutdown_rx: oneshot::Receiver<()>,
) -> Result<(), String> {
    let channel_id = channel.id;
    let channel_name = channel.name.clone();
    let bot_token = channel.bot_token.clone();
    let app_token = channel
        .app_token
        .clone()
        .ok_or_else(|| "Slack channels require an app_token for Socket Mode".to_string())?;

    log::info!("Starting Slack listener for channel: {}", channel_name);

    // Create Slack client
    let client = Arc::new(
        SlackClient::new(SlackClientHyperConnector::new().map_err(|e| e.to_string())?),
    );

    // Create token values
    let _token = SlackApiToken::new(bot_token.into());
    let socket_token = SlackApiToken::new(app_token.into());

    // Emit started event
    broadcaster.broadcast(GatewayEvent::channel_started(
        channel_id,
        ChannelType::Slack.as_str(),
        &channel_name,
    ));

    // Create listener environment
    let listener_environment = Arc::new(
        SlackClientEventsListenerEnvironment::new(client.clone()),
    );

    // Create Socket Mode callbacks with a simple handler
    let socket_mode_callbacks =
        SlackSocketModeListenerCallbacks::new().with_push_events(handle_push_event);

    // Create socket mode listener
    let socket_mode_listener = SlackClientSocketModeListener::new(
        &SlackClientSocketModeConfig::new(),
        listener_environment,
        socket_mode_callbacks,
    );

    // Run with shutdown signal
    let channel_name_for_shutdown = channel_name.clone();
    tokio::select! {
        _ = shutdown_rx => {
            log::info!("Slack listener {} received shutdown signal", channel_name_for_shutdown);
        }
        result = socket_mode_listener.listen_for(&socket_token) => {
            if let Err(e) = result {
                log::error!("Slack listener error: {}", e);
                broadcaster.broadcast(GatewayEvent::channel_error(
                    channel_id,
                    &format!("Slack error: {}", e),
                ));
            }
        }
    }

    // Emit stopped event
    broadcaster.broadcast(GatewayEvent::channel_stopped(
        channel_id,
        ChannelType::Slack.as_str(),
        &channel_name,
    ));

    Ok(())
}

fn handle_push_event(
    event: SlackPushEventCallback,
    _client: Arc<SlackHyperClient>,
    _user_state: SlackClientEventsUserState,
) -> std::pin::Pin<
    Box<dyn std::future::Future<Output = Result<(), Box<dyn std::error::Error + Send + Sync>>> + Send>,
> {
    Box::pin(async move {
        // Log the received event for debugging
        if let SlackEventCallbackBody::Message(msg_event) = event.event {
            // Skip bot messages
            if msg_event.sender.bot_id.is_some() {
                return Ok(());
            }

            if let Some(text) = msg_event.content.as_ref().and_then(|c| c.text.clone()) {
                let user = msg_event
                    .sender
                    .user
                    .as_ref()
                    .map(|u| u.to_string())
                    .unwrap_or_else(|| "unknown".to_string());

                log::info!("Slack message from {}: {}", user, text);

                // TODO: Full integration with dispatcher requires more complex
                // state management through SlackClientEventsUserStateStorage.
                // For now, messages are logged but not processed through AI.
            }
        }

        Ok(())
    })
}

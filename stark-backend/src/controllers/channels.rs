use actix_web::{web, HttpRequest, HttpResponse, Responder};
use serde::Serialize;

use crate::models::{
    get_settings_for_channel_type, ChannelResponse, ChannelSettingsResponse,
    ChannelSettingsSchemaResponse, ChannelType, CreateChannelRequest, UpdateChannelRequest,
    UpdateChannelSettingsRequest,
};
use crate::AppState;

#[derive(Serialize)]
pub struct ChannelsListResponse {
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub channels: Option<Vec<ChannelResponse>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[derive(Serialize)]
pub struct ChannelOperationResponse {
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub channel: Option<ChannelResponse>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

pub fn config(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/api/channels")
            .route("", web::get().to(list_channels))
            .route("", web::post().to(create_channel))
            .route("/settings/schema/{channel_type}", web::get().to(get_settings_schema))
            .route("/{id}", web::get().to(get_channel))
            .route("/{id}", web::put().to(update_channel))
            .route("/{id}", web::delete().to(delete_channel))
            .route("/{id}/start", web::post().to(start_channel))
            .route("/{id}/stop", web::post().to(stop_channel))
            .route("/{id}/settings", web::get().to(get_channel_settings))
            .route("/{id}/settings", web::put().to(update_channel_settings)),
    );
}

fn validate_session_from_request(
    state: &web::Data<AppState>,
    req: &HttpRequest,
) -> Result<(), HttpResponse> {
    let token = req
        .headers()
        .get("Authorization")
        .and_then(|h| h.to_str().ok())
        .map(|s| s.trim_start_matches("Bearer ").to_string());

    let token = match token {
        Some(t) => t,
        None => {
            return Err(HttpResponse::Unauthorized().json(ChannelsListResponse {
                success: false,
                channels: None,
                error: Some("No authorization token provided".to_string()),
            }));
        }
    };

    match state.db.validate_session(&token) {
        Ok(Some(_)) => Ok(()),
        Ok(None) => Err(HttpResponse::Unauthorized().json(ChannelsListResponse {
            success: false,
            channels: None,
            error: Some("Invalid or expired session".to_string()),
        })),
        Err(e) => {
            log::error!("Failed to validate session: {}", e);
            Err(HttpResponse::InternalServerError().json(ChannelsListResponse {
                success: false,
                channels: None,
                error: Some("Internal server error".to_string()),
            }))
        }
    }
}

async fn list_channels(state: web::Data<AppState>, req: HttpRequest) -> impl Responder {
    if let Err(resp) = validate_session_from_request(&state, &req) {
        return resp;
    }

    match state.db.list_channels() {
        Ok(channels) => {
            let channel_manager = state.gateway.channel_manager();
            let responses: Vec<ChannelResponse> = channels
                .into_iter()
                .map(|ch| {
                    let running = channel_manager.is_running(ch.id);
                    ChannelResponse::from(ch).with_running(running)
                })
                .collect();

            HttpResponse::Ok().json(ChannelsListResponse {
                success: true,
                channels: Some(responses),
                error: None,
            })
        }
        Err(e) => {
            log::error!("Failed to list channels: {}", e);
            HttpResponse::InternalServerError().json(ChannelsListResponse {
                success: false,
                channels: None,
                error: Some("Failed to retrieve channels".to_string()),
            })
        }
    }
}

async fn get_channel(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<i64>,
) -> impl Responder {
    if let Err(resp) = validate_session_from_request(&state, &req) {
        return resp;
    }

    let id = path.into_inner();

    match state.db.get_channel(id) {
        Ok(Some(channel)) => {
            let channel_manager = state.gateway.channel_manager();
            let running = channel_manager.is_running(channel.id);
            let response = ChannelResponse::from(channel).with_running(running);

            HttpResponse::Ok().json(ChannelOperationResponse {
                success: true,
                channel: Some(response),
                error: None,
            })
        }
        Ok(None) => HttpResponse::NotFound().json(ChannelOperationResponse {
            success: false,
            channel: None,
            error: Some("Channel not found".to_string()),
        }),
        Err(e) => {
            log::error!("Failed to get channel: {}", e);
            HttpResponse::InternalServerError().json(ChannelOperationResponse {
                success: false,
                channel: None,
                error: Some("Failed to retrieve channel".to_string()),
            })
        }
    }
}

async fn create_channel(
    state: web::Data<AppState>,
    req: HttpRequest,
    body: web::Json<CreateChannelRequest>,
) -> impl Responder {
    if let Err(resp) = validate_session_from_request(&state, &req) {
        return resp;
    }

    // Validate channel type
    if ChannelType::from_str(&body.channel_type).is_none() {
        return HttpResponse::BadRequest().json(ChannelOperationResponse {
            success: false,
            channel: None,
            error: Some("Invalid channel type. Valid options: telegram, slack, discord".to_string()),
        });
    }

    // Validate bot token is not empty
    if body.bot_token.trim().is_empty() {
        return HttpResponse::BadRequest().json(ChannelOperationResponse {
            success: false,
            channel: None,
            error: Some("Bot token cannot be empty".to_string()),
        });
    }

    // Validate name is not empty
    if body.name.trim().is_empty() {
        return HttpResponse::BadRequest().json(ChannelOperationResponse {
            success: false,
            channel: None,
            error: Some("Channel name cannot be empty".to_string()),
        });
    }

    // Slack requires app_token
    if body.channel_type == "slack" && body.app_token.as_ref().map_or(true, |t| t.trim().is_empty())
    {
        return HttpResponse::BadRequest().json(ChannelOperationResponse {
            success: false,
            channel: None,
            error: Some("Slack channels require an app_token for Socket Mode".to_string()),
        });
    }

    match state.db.create_channel(
        &body.channel_type,
        &body.name,
        &body.bot_token,
        body.app_token.as_deref(),
    ) {
        Ok(channel) => HttpResponse::Created().json(ChannelOperationResponse {
            success: true,
            channel: Some(channel.into()),
            error: None,
        }),
        Err(e) => {
            log::error!("Failed to create channel: {}", e);

            // Check for unique constraint violation
            let error_msg = if e.to_string().contains("UNIQUE constraint failed") {
                "A channel with this type and name already exists".to_string()
            } else {
                "Failed to create channel".to_string()
            };

            HttpResponse::BadRequest().json(ChannelOperationResponse {
                success: false,
                channel: None,
                error: Some(error_msg),
            })
        }
    }
}

async fn update_channel(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<i64>,
    body: web::Json<UpdateChannelRequest>,
) -> impl Responder {
    if let Err(resp) = validate_session_from_request(&state, &req) {
        return resp;
    }

    let id = path.into_inner();

    // Validate name if provided
    if let Some(ref name) = body.name {
        if name.trim().is_empty() {
            return HttpResponse::BadRequest().json(ChannelOperationResponse {
                success: false,
                channel: None,
                error: Some("Channel name cannot be empty".to_string()),
            });
        }
    }

    // Validate bot_token if provided
    if let Some(ref token) = body.bot_token {
        if token.trim().is_empty() {
            return HttpResponse::BadRequest().json(ChannelOperationResponse {
                success: false,
                channel: None,
                error: Some("Bot token cannot be empty".to_string()),
            });
        }
    }

    // Handle app_token: None means don't update, Some(value) means set to value
    let app_token_update: Option<Option<&str>> = body.app_token.as_ref().map(|t| Some(t.as_str()));

    match state.db.update_channel(
        id,
        body.name.as_deref(),
        body.enabled,
        body.bot_token.as_deref(),
        app_token_update,
    ) {
        Ok(Some(channel)) => {
            let channel_manager = state.gateway.channel_manager();
            let running = channel_manager.is_running(channel.id);
            let response = ChannelResponse::from(channel).with_running(running);

            HttpResponse::Ok().json(ChannelOperationResponse {
                success: true,
                channel: Some(response),
                error: None,
            })
        }
        Ok(None) => HttpResponse::NotFound().json(ChannelOperationResponse {
            success: false,
            channel: None,
            error: Some("Channel not found".to_string()),
        }),
        Err(e) => {
            log::error!("Failed to update channel: {}", e);
            HttpResponse::InternalServerError().json(ChannelOperationResponse {
                success: false,
                channel: None,
                error: Some("Failed to update channel".to_string()),
            })
        }
    }
}

async fn delete_channel(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<i64>,
) -> impl Responder {
    if let Err(resp) = validate_session_from_request(&state, &req) {
        return resp;
    }

    let id = path.into_inner();

    // Stop the channel if it's running
    let channel_manager = state.gateway.channel_manager();
    if channel_manager.is_running(id) {
        let _ = channel_manager.stop_channel(id).await;
    }

    match state.db.delete_channel(id) {
        Ok(deleted) => {
            if deleted {
                HttpResponse::Ok().json(ChannelOperationResponse {
                    success: true,
                    channel: None,
                    error: None,
                })
            } else {
                HttpResponse::NotFound().json(ChannelOperationResponse {
                    success: false,
                    channel: None,
                    error: Some("Channel not found".to_string()),
                })
            }
        }
        Err(e) => {
            log::error!("Failed to delete channel: {}", e);
            HttpResponse::InternalServerError().json(ChannelOperationResponse {
                success: false,
                channel: None,
                error: Some("Failed to delete channel".to_string()),
            })
        }
    }
}

async fn start_channel(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<i64>,
) -> impl Responder {
    if let Err(resp) = validate_session_from_request(&state, &req) {
        return resp;
    }

    let id = path.into_inner();

    // Get channel from database
    let channel = match state.db.get_channel(id) {
        Ok(Some(ch)) => ch,
        Ok(None) => {
            return HttpResponse::NotFound().json(ChannelOperationResponse {
                success: false,
                channel: None,
                error: Some("Channel not found".to_string()),
            });
        }
        Err(e) => {
            log::error!("Failed to get channel: {}", e);
            return HttpResponse::InternalServerError().json(ChannelOperationResponse {
                success: false,
                channel: None,
                error: Some("Failed to retrieve channel".to_string()),
            });
        }
    };

    // Start the channel
    let channel_manager = state.gateway.channel_manager();
    match channel_manager.start_channel(channel.clone()).await {
        Ok(()) => {
            // Update enabled status in database
            let _ = state.db.set_channel_enabled(id, true);

            let response = ChannelResponse::from(channel).with_running(true);
            HttpResponse::Ok().json(ChannelOperationResponse {
                success: true,
                channel: Some(response),
                error: None,
            })
        }
        Err(e) => {
            log::error!("Failed to start channel: {}", e);
            HttpResponse::BadRequest().json(ChannelOperationResponse {
                success: false,
                channel: None,
                error: Some(e),
            })
        }
    }
}

async fn stop_channel(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<i64>,
) -> impl Responder {
    if let Err(resp) = validate_session_from_request(&state, &req) {
        return resp;
    }

    let id = path.into_inner();

    // Get channel from database
    let channel = match state.db.get_channel(id) {
        Ok(Some(ch)) => ch,
        Ok(None) => {
            return HttpResponse::NotFound().json(ChannelOperationResponse {
                success: false,
                channel: None,
                error: Some("Channel not found".to_string()),
            });
        }
        Err(e) => {
            log::error!("Failed to get channel: {}", e);
            return HttpResponse::InternalServerError().json(ChannelOperationResponse {
                success: false,
                channel: None,
                error: Some("Failed to retrieve channel".to_string()),
            });
        }
    };

    // Stop the channel
    let channel_manager = state.gateway.channel_manager();
    match channel_manager.stop_channel(id).await {
        Ok(()) => {
            // Update enabled status in database
            let _ = state.db.set_channel_enabled(id, false);

            let response = ChannelResponse::from(channel).with_running(false);
            HttpResponse::Ok().json(ChannelOperationResponse {
                success: true,
                channel: Some(response),
                error: None,
            })
        }
        Err(e) => {
            log::error!("Failed to stop channel: {}", e);
            HttpResponse::BadRequest().json(ChannelOperationResponse {
                success: false,
                channel: None,
                error: Some(e),
            })
        }
    }
}

/// Get available settings schema for a channel type
async fn get_settings_schema(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<String>,
) -> impl Responder {
    if let Err(resp) = validate_session_from_request(&state, &req) {
        return resp;
    }

    let channel_type_str = path.into_inner();
    let channel_type = match ChannelType::from_str(&channel_type_str) {
        Some(ct) => ct,
        None => {
            return HttpResponse::BadRequest().json(ChannelSettingsSchemaResponse {
                success: false,
                channel_type: channel_type_str,
                settings: vec![],
            });
        }
    };

    let settings = get_settings_for_channel_type(channel_type);

    HttpResponse::Ok().json(ChannelSettingsSchemaResponse {
        success: true,
        channel_type: channel_type_str,
        settings,
    })
}

/// Get settings for a channel
async fn get_channel_settings(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<i64>,
) -> impl Responder {
    if let Err(resp) = validate_session_from_request(&state, &req) {
        return resp;
    }

    let id = path.into_inner();

    // Verify channel exists
    match state.db.get_channel(id) {
        Ok(Some(_)) => {}
        Ok(None) => {
            return HttpResponse::NotFound().json(ChannelSettingsResponse {
                success: false,
                settings: vec![],
            });
        }
        Err(e) => {
            log::error!("Failed to get channel: {}", e);
            return HttpResponse::InternalServerError().json(ChannelSettingsResponse {
                success: false,
                settings: vec![],
            });
        }
    }

    match state.db.get_channel_settings(id) {
        Ok(settings) => HttpResponse::Ok().json(ChannelSettingsResponse {
            success: true,
            settings,
        }),
        Err(e) => {
            log::error!("Failed to get channel settings: {}", e);
            HttpResponse::InternalServerError().json(ChannelSettingsResponse {
                success: false,
                settings: vec![],
            })
        }
    }
}

/// Update settings for a channel
async fn update_channel_settings(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<i64>,
    body: web::Json<UpdateChannelSettingsRequest>,
) -> impl Responder {
    if let Err(resp) = validate_session_from_request(&state, &req) {
        return resp;
    }

    let id = path.into_inner();

    // Verify channel exists
    match state.db.get_channel(id) {
        Ok(Some(_)) => {}
        Ok(None) => {
            return HttpResponse::NotFound().json(ChannelSettingsResponse {
                success: false,
                settings: vec![],
            });
        }
        Err(e) => {
            log::error!("Failed to get channel: {}", e);
            return HttpResponse::InternalServerError().json(ChannelSettingsResponse {
                success: false,
                settings: vec![],
            });
        }
    }

    // Convert to tuple format for bulk update
    let settings_tuples: Vec<(String, String)> = body
        .settings
        .iter()
        .map(|s| (s.key.clone(), s.value.clone()))
        .collect();

    match state.db.update_channel_settings(id, &settings_tuples) {
        Ok(()) => {
            // Return updated settings
            match state.db.get_channel_settings(id) {
                Ok(settings) => HttpResponse::Ok().json(ChannelSettingsResponse {
                    success: true,
                    settings,
                }),
                Err(e) => {
                    log::error!("Failed to get updated channel settings: {}", e);
                    HttpResponse::InternalServerError().json(ChannelSettingsResponse {
                        success: false,
                        settings: vec![],
                    })
                }
            }
        }
        Err(e) => {
            log::error!("Failed to update channel settings: {}", e);
            HttpResponse::InternalServerError().json(ChannelSettingsResponse {
                success: false,
                settings: vec![],
            })
        }
    }
}

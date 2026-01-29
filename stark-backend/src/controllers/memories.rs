use actix_web::{web, HttpRequest, HttpResponse, Responder};
use chrono::Utc;
use serde::{Deserialize, Serialize};

use crate::models::{CreateMemoryRequest, MemoryResponse, MemoryType, SearchMemoriesRequest, UpdateMemoryRequest, MergeMemoriesRequest};
use crate::AppState;

/// Validate session token from request
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
            return Err(HttpResponse::Unauthorized().json(serde_json::json!({
                "error": "No authorization token provided"
            })));
        }
    };

    match state.db.validate_session(&token) {
        Ok(Some(_)) => Ok(()),
        Ok(None) => Err(HttpResponse::Unauthorized().json(serde_json::json!({
            "error": "Invalid or expired session"
        }))),
        Err(e) => {
            log::error!("Session validation error: {}", e);
            Err(HttpResponse::InternalServerError().json(serde_json::json!({
                "error": "Internal server error"
            })))
        }
    }
}

/// List all memories
async fn list_memories(
    data: web::Data<AppState>,
    req: HttpRequest,
) -> impl Responder {
    if let Err(resp) = validate_session_from_request(&data, &req) {
        return resp;
    }

    match data.db.list_memories() {
        Ok(memories) => {
            let responses: Vec<MemoryResponse> = memories.into_iter().map(|m| m.into()).collect();
            HttpResponse::Ok().json(responses)
        }
        Err(e) => {
            log::error!("Failed to list memories: {}", e);
            HttpResponse::InternalServerError().json(serde_json::json!({
                "error": format!("Database error: {}", e)
            }))
        }
    }
}

/// Create a new memory
async fn create_memory(
    data: web::Data<AppState>,
    req: HttpRequest,
    body: web::Json<CreateMemoryRequest>,
) -> impl Responder {
    if let Err(resp) = validate_session_from_request(&data, &req) {
        return resp;
    }
    // For daily logs, set log_date to today if not provided
    let log_date = if body.memory_type == MemoryType::DailyLog {
        body.log_date.or_else(|| Some(Utc::now().date_naive()))
    } else {
        body.log_date
    };

    match data.db.create_memory(
        body.memory_type,
        &body.content,
        body.category.as_deref(),
        body.tags.as_deref(),
        body.importance,
        body.identity_id.as_deref(),
        body.session_id,
        body.source_channel_type.as_deref(),
        body.source_message_id.as_deref(),
        log_date,
        body.expires_at,
    ) {
        Ok(memory) => {
            let response: MemoryResponse = memory.into();
            HttpResponse::Created().json(response)
        }
        Err(e) => {
            log::error!("Failed to create memory: {}", e);
            HttpResponse::InternalServerError().json(serde_json::json!({
                "error": format!("Database error: {}", e)
            }))
        }
    }
}

/// Search memories using FTS5
async fn search_memories(
    data: web::Data<AppState>,
    req: HttpRequest,
    body: web::Json<SearchMemoriesRequest>,
) -> impl Responder {
    if let Err(resp) = validate_session_from_request(&data, &req) {
        return resp;
    }
    match data.db.search_memories(
        &body.query,
        body.memory_type,
        body.identity_id.as_deref(),
        body.category.as_deref(),
        body.min_importance,
        body.limit,
    ) {
        Ok(results) => HttpResponse::Ok().json(results),
        Err(e) => {
            log::error!("Failed to search memories: {}", e);
            HttpResponse::InternalServerError().json(serde_json::json!({
                "error": format!("Database error: {}", e)
            }))
        }
    }
}

/// Get today's daily logs
#[derive(Deserialize)]
struct DailyLogsQuery {
    identity_id: Option<String>,
}

async fn get_daily_logs(
    data: web::Data<AppState>,
    req: HttpRequest,
    query: web::Query<DailyLogsQuery>,
) -> impl Responder {
    if let Err(resp) = validate_session_from_request(&data, &req) {
        return resp;
    }
    match data.db.get_todays_daily_logs(query.identity_id.as_deref()) {
        Ok(memories) => {
            let responses: Vec<MemoryResponse> = memories.into_iter().map(|m| m.into()).collect();
            HttpResponse::Ok().json(responses)
        }
        Err(e) => {
            log::error!("Failed to get daily logs: {}", e);
            HttpResponse::InternalServerError().json(serde_json::json!({
                "error": format!("Database error: {}", e)
            }))
        }
    }
}

/// Get long-term memories
#[derive(Deserialize)]
struct LongTermQuery {
    identity_id: Option<String>,
    min_importance: Option<i32>,
    #[serde(default = "default_limit")]
    limit: i32,
}

fn default_limit() -> i32 {
    20
}

async fn get_long_term_memories(
    data: web::Data<AppState>,
    req: HttpRequest,
    query: web::Query<LongTermQuery>,
) -> impl Responder {
    if let Err(resp) = validate_session_from_request(&data, &req) {
        return resp;
    }
    match data.db.get_long_term_memories(
        query.identity_id.as_deref(),
        query.min_importance,
        query.limit,
    ) {
        Ok(memories) => {
            let responses: Vec<MemoryResponse> = memories.into_iter().map(|m| m.into()).collect();
            HttpResponse::Ok().json(responses)
        }
        Err(e) => {
            log::error!("Failed to get long-term memories: {}", e);
            HttpResponse::InternalServerError().json(serde_json::json!({
                "error": format!("Database error: {}", e)
            }))
        }
    }
}

/// Delete a memory
async fn delete_memory(
    data: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<i64>,
) -> impl Responder {
    if let Err(resp) = validate_session_from_request(&data, &req) {
        return resp;
    }
    let memory_id = path.into_inner();

    match data.db.delete_memory(memory_id) {
        Ok(true) => HttpResponse::Ok().json(serde_json::json!({
            "success": true,
            "message": "Memory deleted"
        })),
        Ok(false) => HttpResponse::NotFound().json(serde_json::json!({
            "error": "Memory not found"
        })),
        Err(e) => {
            log::error!("Failed to delete memory: {}", e);
            HttpResponse::InternalServerError().json(serde_json::json!({
                "error": format!("Database error: {}", e)
            }))
        }
    }
}

/// Cleanup expired memories
async fn cleanup_expired(data: web::Data<AppState>, req: HttpRequest) -> impl Responder {
    if let Err(resp) = validate_session_from_request(&data, &req) {
        return resp;
    }
    match data.db.cleanup_expired_memories() {
        Ok(count) => HttpResponse::Ok().json(serde_json::json!({
            "success": true,
            "deleted_count": count
        })),
        Err(e) => {
            log::error!("Failed to cleanup expired memories: {}", e);
            HttpResponse::InternalServerError().json(serde_json::json!({
                "error": format!("Database error: {}", e)
            }))
        }
    }
}

// =====================================================
// Phase 5: Enhanced Memory Browser API Endpoints
// =====================================================

/// Get a single memory by ID
async fn get_memory(
    data: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<i64>,
) -> impl Responder {
    if let Err(resp) = validate_session_from_request(&data, &req) {
        return resp;
    }
    let memory_id = path.into_inner();

    match data.db.get_memory(memory_id) {
        Ok(Some(memory)) => {
            let response: MemoryResponse = memory.into();
            HttpResponse::Ok().json(response)
        }
        Ok(None) => HttpResponse::NotFound().json(serde_json::json!({
            "error": "Memory not found"
        })),
        Err(e) => {
            log::error!("Failed to get memory: {}", e);
            HttpResponse::InternalServerError().json(serde_json::json!({
                "error": format!("Database error: {}", e)
            }))
        }
    }
}

/// Update a memory
async fn update_memory(
    data: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<i64>,
    body: web::Json<UpdateMemoryRequest>,
) -> impl Responder {
    if let Err(resp) = validate_session_from_request(&data, &req) {
        return resp;
    }
    let memory_id = path.into_inner();

    match data.db.update_memory(memory_id, &body.into_inner()) {
        Ok(Some(memory)) => {
            let response: MemoryResponse = memory.into();
            HttpResponse::Ok().json(response)
        }
        Ok(None) => HttpResponse::NotFound().json(serde_json::json!({
            "error": "Memory not found"
        })),
        Err(e) => {
            log::error!("Failed to update memory: {}", e);
            HttpResponse::InternalServerError().json(serde_json::json!({
                "error": format!("Database error: {}", e)
            }))
        }
    }
}

/// Merge multiple memories
async fn merge_memories(
    data: web::Data<AppState>,
    req: HttpRequest,
    body: web::Json<MergeMemoriesRequest>,
) -> impl Responder {
    if let Err(resp) = validate_session_from_request(&data, &req) {
        return resp;
    }

    if body.memory_ids.len() < 2 {
        return HttpResponse::BadRequest().json(serde_json::json!({
            "error": "At least 2 memory IDs required for merge"
        }));
    }

    // Get memories to merge
    let mut memories = Vec::new();
    let mut max_importance = 5;
    let mut identity_id = None;
    let mut memory_type = MemoryType::LongTerm;

    for id in &body.memory_ids {
        match data.db.get_memory(*id) {
            Ok(Some(mem)) => {
                if body.use_max_importance.unwrap_or(true) && mem.importance > max_importance {
                    max_importance = mem.importance;
                }
                if identity_id.is_none() {
                    identity_id = mem.identity_id.clone();
                }
                memory_type = mem.memory_type;
                memories.push(mem);
            }
            Ok(None) => {
                return HttpResponse::NotFound().json(serde_json::json!({
                    "error": format!("Memory {} not found", id)
                }));
            }
            Err(e) => {
                return HttpResponse::InternalServerError().json(serde_json::json!({
                    "error": format!("Database error: {}", e)
                }));
            }
        }
    }

    // Create the merged memory
    match data.db.create_memory_extended(
        memory_type,
        &body.merged_content,
        Some("merged"),
        None,
        max_importance,
        identity_id.as_deref(),
        None, None, None, None, None,
        memories[0].entity_type.as_deref(),
        memories[0].entity_name.as_deref(),
        Some(1.0),
        Some("merged"),
        None, None, None,
    ) {
        Ok(merged) => {
            // Mark original memories as superseded
            for mem in &memories {
                let _ = data.db.supersede_memory(mem.id, merged.id);
            }

            let response: MemoryResponse = merged.into();
            HttpResponse::Created().json(serde_json::json!({
                "success": true,
                "merged_memory": response,
                "superseded_count": memories.len()
            }))
        }
        Err(e) => {
            log::error!("Failed to create merged memory: {}", e);
            HttpResponse::InternalServerError().json(serde_json::json!({
                "error": format!("Database error: {}", e)
            }))
        }
    }
}

/// Get memory statistics
async fn get_stats(
    data: web::Data<AppState>,
    req: HttpRequest,
) -> impl Responder {
    if let Err(resp) = validate_session_from_request(&data, &req) {
        return resp;
    }

    match data.db.get_memory_stats() {
        Ok(stats) => HttpResponse::Ok().json(stats),
        Err(e) => {
            log::error!("Failed to get memory stats: {}", e);
            HttpResponse::InternalServerError().json(serde_json::json!({
                "error": format!("Database error: {}", e)
            }))
        }
    }
}

/// Export memories as markdown
#[derive(Deserialize)]
struct ExportQuery {
    identity_id: Option<String>,
}

async fn export_memories(
    data: web::Data<AppState>,
    req: HttpRequest,
    query: web::Query<ExportQuery>,
) -> impl Responder {
    if let Err(resp) = validate_session_from_request(&data, &req) {
        return resp;
    }

    match data.db.export_memories_markdown(query.identity_id.as_deref()) {
        Ok(markdown) => HttpResponse::Ok()
            .content_type("text/markdown")
            .insert_header(("Content-Disposition", "attachment; filename=\"memories.md\""))
            .body(markdown),
        Err(e) => {
            log::error!("Failed to export memories: {}", e);
            HttpResponse::InternalServerError().json(serde_json::json!({
                "error": format!("Database error: {}", e)
            }))
        }
    }
}

/// List memories with filters (Phase 5)
#[derive(Deserialize)]
struct ListMemoriesQuery {
    memory_type: Option<String>,
    identity_id: Option<String>,
    min_importance: Option<i32>,
    include_superseded: Option<bool>,
    #[serde(default = "default_limit")]
    limit: i32,
    #[serde(default)]
    offset: i32,
}

async fn list_memories_filtered(
    data: web::Data<AppState>,
    req: HttpRequest,
    query: web::Query<ListMemoriesQuery>,
) -> impl Responder {
    if let Err(resp) = validate_session_from_request(&data, &req) {
        return resp;
    }

    let memory_type = query.memory_type.as_ref().and_then(|t| MemoryType::from_str(t));

    match data.db.list_memories_filtered(
        memory_type,
        query.identity_id.as_deref(),
        query.min_importance,
        query.include_superseded.unwrap_or(false),
        query.limit,
        query.offset,
    ) {
        Ok(memories) => {
            let responses: Vec<MemoryResponse> = memories.into_iter().map(|m| m.into()).collect();
            HttpResponse::Ok().json(responses)
        }
        Err(e) => {
            log::error!("Failed to list memories: {}", e);
            HttpResponse::InternalServerError().json(serde_json::json!({
                "error": format!("Database error: {}", e)
            }))
        }
    }
}

pub fn config(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/api/memories")
            .route("", web::get().to(list_memories))
            .route("", web::post().to(create_memory))
            .route("/search", web::post().to(search_memories))
            .route("/daily", web::get().to(get_daily_logs))
            .route("/long-term", web::get().to(get_long_term_memories))
            .route("/cleanup", web::post().to(cleanup_expired))
            // Phase 5: Enhanced endpoints
            .route("/filtered", web::get().to(list_memories_filtered))
            .route("/merge", web::post().to(merge_memories))
            .route("/stats", web::get().to(get_stats))
            .route("/export", web::get().to(export_memories))
            .route("/{id}", web::get().to(get_memory))
            .route("/{id}", web::put().to(update_memory))
            .route("/{id}", web::delete().to(delete_memory)),
    );
}

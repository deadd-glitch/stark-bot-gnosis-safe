use actix_web::{web, HttpRequest, HttpResponse, Responder};
use serde::{Deserialize, Serialize};

use crate::skills::{Skill, SkillMetadata};
use crate::AppState;

#[derive(Serialize)]
pub struct SkillsListResponse {
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub skills: Option<Vec<SkillInfo>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[derive(Serialize)]
pub struct SkillInfo {
    pub name: String,
    pub description: String,
    pub version: String,
    pub source: String,
    pub enabled: bool,
    pub requires_tools: Vec<String>,
    pub requires_binaries: Vec<String>,
    pub tags: Vec<String>,
}

impl From<&Skill> for SkillInfo {
    fn from(skill: &Skill) -> Self {
        SkillInfo {
            name: skill.metadata.name.clone(),
            description: skill.metadata.description.clone(),
            version: skill.metadata.version.clone(),
            source: skill.source.as_str().to_string(),
            enabled: skill.enabled,
            requires_tools: skill.metadata.requires_tools.clone(),
            requires_binaries: skill.metadata.requires_binaries.clone(),
            tags: skill.metadata.tags.clone(),
        }
    }
}

#[derive(Serialize)]
pub struct SkillDetailResponse {
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub skill: Option<SkillDetail>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[derive(Serialize)]
pub struct SkillDetail {
    pub name: String,
    pub description: String,
    pub version: String,
    pub source: String,
    pub path: String,
    pub enabled: bool,
    pub requires_tools: Vec<String>,
    pub requires_binaries: Vec<String>,
    pub missing_binaries: Vec<String>,
    pub tags: Vec<String>,
    pub arguments: Vec<ArgumentInfo>,
    pub prompt_template: String,
}

#[derive(Serialize)]
pub struct ArgumentInfo {
    pub name: String,
    pub description: String,
    pub required: bool,
    pub default: Option<String>,
}

impl From<&Skill> for SkillDetail {
    fn from(skill: &Skill) -> Self {
        let missing_binaries = skill.check_binaries().err().unwrap_or_default();

        let arguments: Vec<ArgumentInfo> = skill
            .metadata
            .arguments
            .iter()
            .map(|(name, arg)| ArgumentInfo {
                name: name.clone(),
                description: arg.description.clone(),
                required: arg.required,
                default: arg.default.clone(),
            })
            .collect();

        SkillDetail {
            name: skill.metadata.name.clone(),
            description: skill.metadata.description.clone(),
            version: skill.metadata.version.clone(),
            source: skill.source.as_str().to_string(),
            path: skill.path.clone(),
            enabled: skill.enabled,
            requires_tools: skill.metadata.requires_tools.clone(),
            requires_binaries: skill.metadata.requires_binaries.clone(),
            missing_binaries,
            tags: skill.metadata.tags.clone(),
            arguments,
            prompt_template: skill.prompt_template.clone(),
        }
    }
}

#[derive(Deserialize)]
pub struct SetEnabledRequest {
    pub enabled: bool,
}

#[derive(Serialize)]
pub struct OperationResponse {
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

pub fn config(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/api/skills")
            .route("", web::get().to(list_skills))
            .route("/{name}", web::get().to(get_skill))
            .route("/{name}/enabled", web::put().to(set_enabled))
            .route("/reload", web::post().to(reload_skills)),
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
            return Err(HttpResponse::Unauthorized().json(SkillsListResponse {
                success: false,
                skills: None,
                error: Some("No authorization token provided".to_string()),
            }));
        }
    };

    match state.db.validate_session(&token) {
        Ok(Some(_)) => Ok(()),
        Ok(None) => Err(HttpResponse::Unauthorized().json(SkillsListResponse {
            success: false,
            skills: None,
            error: Some("Invalid or expired session".to_string()),
        })),
        Err(e) => {
            log::error!("Failed to validate session: {}", e);
            Err(HttpResponse::InternalServerError().json(SkillsListResponse {
                success: false,
                skills: None,
                error: Some("Internal server error".to_string()),
            }))
        }
    }
}

async fn list_skills(state: web::Data<AppState>, req: HttpRequest) -> impl Responder {
    if let Err(resp) = validate_session_from_request(&state, &req) {
        return resp;
    }

    let skills: Vec<SkillInfo> = state
        .skill_registry
        .list()
        .iter()
        .map(|s| s.into())
        .collect();

    HttpResponse::Ok().json(SkillsListResponse {
        success: true,
        skills: Some(skills),
        error: None,
    })
}

async fn get_skill(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<String>,
) -> impl Responder {
    if let Err(resp) = validate_session_from_request(&state, &req) {
        return resp;
    }

    let name = path.into_inner();

    match state.skill_registry.get(&name) {
        Some(skill) => HttpResponse::Ok().json(SkillDetailResponse {
            success: true,
            skill: Some((&skill).into()),
            error: None,
        }),
        None => HttpResponse::NotFound().json(SkillDetailResponse {
            success: false,
            skill: None,
            error: Some(format!("Skill '{}' not found", name)),
        }),
    }
}

async fn set_enabled(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<String>,
    body: web::Json<SetEnabledRequest>,
) -> impl Responder {
    if let Err(resp) = validate_session_from_request(&state, &req) {
        return resp;
    }

    let name = path.into_inner();

    if !state.skill_registry.has_skill(&name) {
        return HttpResponse::NotFound().json(OperationResponse {
            success: false,
            message: None,
            error: Some(format!("Skill '{}' not found", name)),
        });
    }

    // Update in registry
    state.skill_registry.set_enabled(&name, body.enabled);

    // Update in database
    if let Err(e) = state.db.set_skill_enabled(&name, body.enabled) {
        log::warn!("Failed to update skill enabled status in database: {}", e);
    }

    let status = if body.enabled { "enabled" } else { "disabled" };
    HttpResponse::Ok().json(OperationResponse {
        success: true,
        message: Some(format!("Skill '{}' {}", name, status)),
        error: None,
    })
}

async fn reload_skills(state: web::Data<AppState>, req: HttpRequest) -> impl Responder {
    if let Err(resp) = validate_session_from_request(&state, &req) {
        return resp;
    }

    match state.skill_registry.reload().await {
        Ok(count) => HttpResponse::Ok().json(OperationResponse {
            success: true,
            message: Some(format!("Loaded {} skills", count)),
            error: None,
        }),
        Err(e) => {
            log::error!("Failed to reload skills: {}", e);
            HttpResponse::InternalServerError().json(OperationResponse {
                success: false,
                message: None,
                error: Some(format!("Failed to reload skills: {}", e)),
            })
        }
    }
}

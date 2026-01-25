use actix_cors::Cors;
use actix_files::Files;
use actix_web::{middleware::Logger, web, App, HttpServer};
use dotenv::dotenv;
use std::sync::Arc;

mod ai;
mod channels;
mod config;
mod controllers;
mod db;
mod gateway;
mod middleware;
mod models;
mod skills;
mod tools;

use config::Config;
use db::Database;
use gateway::Gateway;
use skills::SkillRegistry;
use tools::ToolRegistry;

pub struct AppState {
    pub db: Arc<Database>,
    pub config: Config,
    pub gateway: Arc<Gateway>,
    pub tool_registry: Arc<ToolRegistry>,
    pub skill_registry: Arc<SkillRegistry>,
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenv().ok();
    env_logger::init();

    let config = Config::from_env();
    let port = config.port;
    let gateway_port = config.gateway_port;

    log::info!("Initializing database at {}", config.database_url);
    let db = Database::new(&config.database_url).expect("Failed to initialize database");
    let db = Arc::new(db);

    // Initialize Tool Registry with built-in tools
    log::info!("Initializing tool registry");
    let tool_registry = Arc::new(tools::create_default_registry());
    log::info!("Registered {} tools", tool_registry.len());

    // Initialize Skill Registry
    log::info!("Initializing skill registry");
    let skill_registry = Arc::new(skills::create_default_registry());

    // Load skills from disk
    let skill_count = skill_registry.load_all().await.unwrap_or_else(|e| {
        log::warn!("Failed to load skills: {}", e);
        0
    });
    log::info!("Loaded {} skills", skill_count);

    // Initialize Gateway with tool registry
    log::info!("Initializing Gateway");
    let gateway = Arc::new(Gateway::new_with_tools(db.clone(), tool_registry.clone()));

    // Start Gateway WebSocket server
    let gw = gateway.clone();
    tokio::spawn(async move {
        gw.start(gateway_port).await;
    });

    // Start enabled channels
    log::info!("Starting enabled channels");
    gateway.start_enabled_channels().await;

    log::info!("Starting StarkBot server on port {}", port);
    log::info!("Gateway WebSocket server on port {}", gateway_port);

    let tool_reg = tool_registry.clone();
    let skill_reg = skill_registry.clone();

    HttpServer::new(move || {
        let cors = Cors::default()
            .allow_any_origin()
            .allow_any_method()
            .allow_any_header()
            .max_age(3600);

        App::new()
            .app_data(web::Data::new(AppState {
                db: Arc::clone(&db),
                config: config.clone(),
                gateway: Arc::clone(&gateway),
                tool_registry: Arc::clone(&tool_reg),
                skill_registry: Arc::clone(&skill_reg),
            }))
            .wrap(Logger::default())
            .wrap(cors)
            .configure(controllers::health::config)
            .configure(controllers::auth::config)
            .configure(controllers::dashboard::config)
            .configure(controllers::chat::config)
            .configure(controllers::api_keys::config)
            .configure(controllers::channels::config)
            .configure(controllers::agent_settings::configure)
            .configure(controllers::sessions::config)
            .configure(controllers::memories::config)
            .configure(controllers::identity::config)
            .configure(controllers::tools::config)
            .configure(controllers::skills::config)
            .service(Files::new("/", "./stark-frontend").index_file("index.html"))
    })
    .bind(("0.0.0.0", port))?
    .run()
    .await
}

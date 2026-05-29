pub mod agents;
pub mod attachments;
pub mod chat;
pub mod conversations;
pub mod memory;
pub mod settings;
pub mod skills;
pub mod templates;

use crate::{
    agents::AgentsService, auth::{AuthState, require_auth}, chat::ChatService,
    conversations::ConversationsService, llm::ProviderRegistry, memory::MemoryService,
    settings::SettingsService, skill_attachments::AttachmentsService, skills::SkillsService,
    templates::TemplatesService,
};
use axum::{
    Router,
    http::{HeaderValue, Method, header},
    middleware::from_fn_with_state,
};
use std::sync::Arc;
use tower_http::cors::CorsLayer;

#[derive(Clone)]
pub struct AppState {
    pub settings: SettingsService,
    pub providers: ProviderRegistry,
    pub agents: AgentsService,
    pub skills: SkillsService,
    pub attachments: AttachmentsService,
    pub templates: TemplatesService,
    pub conversations: ConversationsService,
    pub memory: MemoryService,
    pub chat: ChatService,
    pub auth: AuthState,
}

pub fn router(state: Arc<AppState>, cors_allowed_origin: Option<String>) -> Router {
    // Everything except the health check sits behind `require_auth`.
    let protected = agents::routes()
        .merge(skills::routes())
        .merge(attachments::routes())
        .merge(templates::routes())
        .merge(conversations::routes())
        .merge(memory::routes())
        .merge(chat::routes())
        .merge(settings::routes())
        .layer(from_fn_with_state(state.auth.clone(), require_auth));

    let api = settings::public_routes().merge(protected);

    let mut app = Router::new().nest("/api", api).with_state(state);
    if let Some(cors) = build_cors(cors_allowed_origin) {
        app = app.layer(cors);
    }
    app
}

/// Build a CORS layer permitting the configured cross-origin frontend plus the
/// `Authorization` header. Returns `None` for same-origin deployments (no origin
/// configured) so no layer is applied.
fn build_cors(origin: Option<String>) -> Option<CorsLayer> {
    let value = origin?.parse::<HeaderValue>().ok()?;
    Some(
        CorsLayer::new()
            .allow_origin(value)
            .allow_methods([Method::GET, Method::POST, Method::PUT, Method::DELETE])
            .allow_headers([header::AUTHORIZATION, header::CONTENT_TYPE]),
    )
}

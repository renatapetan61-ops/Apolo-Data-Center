use apolo_ax_scheduler::AxScheduler;
use argon2::{password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString}, Argon2};
use aes_gcm::{aead::{Aead, KeyInit}, Aes256Gcm, Nonce};
use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    response::Html,
    routing::{delete, get, post},
    Json, Router,
};
use rand::{rngs::OsRng, RngCore};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sqlx::{postgres::PgPoolOptions, PgPool};
use std::{env, net::SocketAddr, sync::Arc};
use subtle::ConstantTimeEq;
use time::{Duration, OffsetDateTime};

#[derive(Clone)]
struct AppState {
    scheduler: Arc<AxScheduler>,
    database: Option<PgPool>,
    api_token: Option<String>,
}
#[derive(Serialize)]
struct Overview {
    service: &'static str, status: &'static str, control_plane: &'static str,
    persistence: &'static str, node_count: usize,
}
#[derive(Debug, Serialize, sqlx::FromRow)]
struct Project { id: uuid::Uuid, name: String, description: String, created_at: time::OffsetDateTime }
#[derive(Debug, Deserialize)]
struct CreateProject { name: String, #[serde(default)] description: String }
#[derive(Debug, Deserialize)]
struct RegisterInput { name: String, email: String, password: String }
#[derive(Debug, Deserialize)]
struct LoginInput { email: String, password: String }
#[derive(Debug, Deserialize)]
struct CreateTokenInput { #[serde(default = "default_token_name")] name: String }
fn default_token_name() -> String { "Token da API".into() }
#[derive(Serialize)]
struct AuthResponse { access_token: String, user: UserInfo }
#[derive(Serialize)]
struct UserInfo { id: uuid::Uuid, name: String, email: String }
#[derive(Serialize)]
struct CreatedToken { token: String, name: String, prefix: String, warning: &'static str }
async fn dashboard() -> Html<&'static str> { Html(include_str!("dashboard.html")) }
async fn health(State(state): State<AppState>) -> Json<serde_json::Value> {
    Json(serde_json::json!({"status":"ok","service":"apolo-ax-api","version":env!("CARGO_PKG_VERSION"),
        "database":if state.database.is_some(){"connected"}else{"not_configured"}}))
}
async fn overview(State(state): State<AppState>) -> Result<Json<Overview>, StatusCode> {
    let nodes = state.scheduler.list_nodes().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(Overview { service:"APOLO DATA CENTER", status:"operational", control_plane:"single_process",
        persistence:if state.database.is_some(){"postgresql_projects_auth_scheduler_volatile"}else{"volatile_memory"}, node_count:nodes.len() }))
}
async fn nodes(State(state): State<AppState>) -> Result<Json<Vec<apolo_ax_core::NodeInventory>>, StatusCode> {
    state.scheduler.list_nodes().map(Json).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}
fn database(state: &AppState) -> Result<&PgPool, StatusCode> { state.database.as_ref().ok_or(StatusCode::SERVICE_UNAVAILABLE) }
fn hash_token(token: &str) -> String { format!("{:x}", Sha256::digest(token.as_bytes())) }
fn token_cipher() -> Result<Aes256Gcm, StatusCode> {
    let secret = env::var("APOLO_TOKEN_ENCRYPTION_KEY").map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
    if secret.len() < 32 { return Err(StatusCode::SERVICE_UNAVAILABLE); }
    let key = Sha256::digest(secret.as_bytes());
    Aes256Gcm::new_from_slice(&key).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}
fn encrypt_token(token: &str) -> Result<Vec<u8>, StatusCode> {
    let cipher = token_cipher()?;
    let mut nonce = [0u8; 12];
    OsRng.fill_bytes(&mut nonce);
    let encrypted = cipher.encrypt(Nonce::from_slice(&nonce), token.as_bytes())
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok([nonce.as_slice(), encrypted.as_slice()].concat())
}
fn decrypt_token(payload: &[u8]) -> Result<String, StatusCode> {
    if payload.len() <= 12 { return Err(StatusCode::INTERNAL_SERVER_ERROR); }
    let cipher = token_cipher()?;
    let plain = cipher.decrypt(Nonce::from_slice(&payload[..12]), &payload[12..])
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    String::from_utf8(plain).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}
fn random_token(prefix: &str) -> String {
    let mut bytes = [0u8; 32]; OsRng.fill_bytes(&mut bytes);
    format!("{prefix}{}", bytes.iter().map(|b| format!("{b:02x}")).collect::<String>())
}
async fn register(State(state): State<AppState>, Json(input): Json<RegisterInput>) -> Result<(StatusCode, Json<AuthResponse>), StatusCode> {
    let db = database(&state)?;
    let name = input.name.trim(); let email = input.email.trim().to_lowercase();
    if name.is_empty() || name.len() > 120 || !email.contains('@') || email.len() > 320 || input.password.len() < 10 || input.password.len() > 256 {
        return Err(StatusCode::BAD_REQUEST);
    }
    let salt = SaltString::generate(&mut OsRng);
    let password_hash = Argon2::default().hash_password(input.password.as_bytes(), &salt)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?.to_string();
    let user = sqlx::query_as::<_, (uuid::Uuid, String, String)>(
        "INSERT INTO users (name,email,password_hash) VALUES ($1,$2,$3) RETURNING id,name,email")
        .bind(name).bind(email).bind(password_hash).fetch_one(db).await.map_err(|e| {
            if e.to_string().contains("unique") { StatusCode::CONFLICT } else { StatusCode::INTERNAL_SERVER_ERROR }
        })?;
    create_session(db, user).await
}
async fn login(State(state): State<AppState>, Json(input): Json<LoginInput>) -> Result<Json<AuthResponse>, StatusCode> {
    let db = database(&state)?;
    let user = sqlx::query_as::<_, (uuid::Uuid, String, String, String)>(
        "SELECT id,name,email,password_hash FROM users WHERE email=$1")
        .bind(input.email.trim().to_lowercase()).fetch_optional(db).await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?.ok_or(StatusCode::UNAUTHORIZED)?;
    let parsed = PasswordHash::new(&user.3).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Argon2::default().verify_password(input.password.as_bytes(), &parsed).map_err(|_| StatusCode::UNAUTHORIZED)?;
    create_session(db, (user.0,user.1,user.2)).await.map(|v| v.1)
}
async fn create_session(db: &PgPool, user: (uuid::Uuid,String,String)) -> Result<(StatusCode, Json<AuthResponse>), StatusCode> {
    let token = random_token("apolo_session_");
    sqlx::query("INSERT INTO auth_sessions (user_id,token_hash,expires_at) VALUES ($1,$2,$3)")
        .bind(user.0).bind(hash_token(&token)).bind(OffsetDateTime::now_utc()+Duration::days(30))
        .execute(db).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok((StatusCode::OK, Json(AuthResponse { access_token:token, user:UserInfo{id:user.0,name:user.1,email:user.2} })))
}
async fn create_api_token(State(state): State<AppState>, headers: HeaderMap, Json(input): Json<CreateTokenInput>) -> Result<(StatusCode, Json<CreatedToken>), StatusCode> {
    let db = database(&state)?;
    let user_id = authorize_session(db, &headers).await?;
    let name = input.name.trim();
    if name.is_empty() || name.len() > 120 { return Err(StatusCode::BAD_REQUEST); }
    let token = random_token("apolo_");
    let prefix = token.chars().take(18).collect::<String>();
    let ciphertext = encrypt_token(&token)?;
    sqlx::query("INSERT INTO api_tokens (user_id,name,token_hash,token_prefix,token_ciphertext) VALUES ($1,$2,$3,$4,$5)")
        .bind(user_id).bind(name).bind(hash_token(&token)).bind(&prefix).bind(ciphertext)
        .execute(db).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok((StatusCode::CREATED, Json(CreatedToken { token, name:name.to_string(), prefix, warning:"Token protegido e associado à sua conta." })))
}
async fn get_active_api_token(State(state): State<AppState>, headers: HeaderMap) -> Result<Json<serde_json::Value>, StatusCode> {
    let db = database(&state)?;
    let user_id = authorize_session(db, &headers).await?;
    let row = sqlx::query_as::<_, (String, Option<Vec<u8>>)>(
        "SELECT name,token_ciphertext FROM api_tokens WHERE user_id=$1 ORDER BY created_at DESC LIMIT 1")
        .bind(user_id).fetch_optional(db).await.map_err(|_|StatusCode::INTERNAL_SERVER_ERROR)?;
    match row {
        Some((name, Some(ciphertext))) => Ok(Json(serde_json::json!({"name":name,"token":decrypt_token(&ciphertext)?}))),
        _ => Err(StatusCode::NOT_FOUND)
    }
}
async fn authorize_session(db: &PgPool, headers: &HeaderMap) -> Result<uuid::Uuid, StatusCode> {
    let supplied = bearer(headers)?;
    sqlx::query_scalar::<_,uuid::Uuid>("SELECT user_id FROM auth_sessions WHERE token_hash=$1 AND expires_at>NOW()")
        .bind(hash_token(supplied)).fetch_optional(db).await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?.ok_or(StatusCode::UNAUTHORIZED)
}
fn bearer(headers: &HeaderMap) -> Result<&str, StatusCode> {
    headers.get(axum::http::header::AUTHORIZATION).and_then(|v|v.to_str().ok())
        .and_then(|v|v.strip_prefix("Bearer ")).ok_or(StatusCode::UNAUTHORIZED)
}
async fn authorize(state: &AppState, headers: &HeaderMap) -> Result<Option<uuid::Uuid>, StatusCode> {
    let supplied = bearer(headers)?;
    if let Some(expected) = state.api_token.as_deref() {
        if bool::from(supplied.as_bytes().ct_eq(expected.as_bytes())) { return Ok(None); }
    }
    let db = database(state)?;
    sqlx::query_scalar::<_,uuid::Uuid>("UPDATE api_tokens SET last_used_at=NOW() WHERE token_hash=$1 RETURNING user_id")
        .bind(hash_token(supplied)).fetch_optional(db).await
        .map_err(|_|StatusCode::INTERNAL_SERVER_ERROR)?.map(Some).ok_or(StatusCode::UNAUTHORIZED)
}
async fn list_projects(State(state):State<AppState>,headers:HeaderMap)->Result<Json<Vec<Project>>,StatusCode>{
    let owner=authorize(&state,&headers).await?;
    let rows=if let Some(user_id)=owner {
        sqlx::query_as::<_,Project>("SELECT id,name,description,created_at FROM projects WHERE user_id=$1 ORDER BY created_at DESC")
            .bind(user_id).fetch_all(database(&state)?).await
    } else {
        sqlx::query_as::<_,Project>("SELECT id,name,description,created_at FROM projects ORDER BY created_at DESC")
            .fetch_all(database(&state)?).await
    }.map_err(|_|StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(rows))
}
async fn create_project(State(state):State<AppState>,headers:HeaderMap,Json(input):Json<CreateProject>)->Result<(StatusCode,Json<Project>),StatusCode>{
    let owner=authorize(&state,&headers).await?;
    let name=input.name.trim();
    if name.is_empty()||name.len()>120||input.description.len()>4000{return Err(StatusCode::BAD_REQUEST)}
    let project=sqlx::query_as::<_,Project>("INSERT INTO projects(name,description,user_id) VALUES($1,$2,$3) RETURNING id,name,description,created_at")
        .bind(name).bind(input.description.trim()).bind(owner)
        .fetch_one(database(&state)?).await.map_err(|_|StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok((StatusCode::CREATED,Json(project)))
}
async fn delete_project(State(state):State<AppState>,Path(id):Path<uuid::Uuid>,headers:HeaderMap)->Result<StatusCode,StatusCode>{
    let owner=authorize(&state,&headers).await?;
    let result=if let Some(user_id)=owner {
        sqlx::query("DELETE FROM projects WHERE id=$1 AND user_id=$2").bind(id).bind(user_id).execute(database(&state)?).await
    } else {
        sqlx::query("DELETE FROM projects WHERE id=$1").bind(id).execute(database(&state)?).await
    }.map_err(|_|StatusCode::INTERNAL_SERVER_ERROR)?;
    if result.rows_affected()==0{return Err(StatusCode::NOT_FOUND)} Ok(StatusCode::NO_CONTENT)
}
fn app(state:AppState)->Router{
    Router::new().route("/",get(dashboard)).route("/healthz",get(health))
        .route("/api/v1/overview",get(overview)).route("/api/v1/nodes",get(nodes))
        .route("/api/v1/auth/register",post(register)).route("/api/v1/auth/login",post(login))
        .route("/api/v1/tokens",post(create_api_token).get(get_active_api_token))
        .route("/api/v1/projects",get(list_projects).post(create_project))
        .route("/api/v1/projects/{project_id}",delete(delete_project)).with_state(state)
}
#[tokio::main]
async fn main()->Result<(),Box<dyn std::error::Error>>{
    let port=env::var("PORT").unwrap_or_else(|_|"10000".into());
    let bind=env::var("APOLO_BIND_ADDR").unwrap_or_else(|_|format!("0.0.0.0:{port}"));
    let address:SocketAddr=bind.parse()?;
    let database=if let Ok(url)=env::var("DATABASE_URL"){
        let pool=PgPoolOptions::new().max_connections(10).connect(&url).await?;
        sqlx::migrate!("./migrations").run(&pool).await?;Some(pool)
    }else{eprintln!("DATABASE_URL not configured; persistent account and project API disabled");None};
    let api_token=env::var("APOLO_API_TOKEN").ok().filter(|v|v.len()>=32);
    if api_token.is_none(){eprintln!("APOLO_API_TOKEN missing; legacy static token disabled");}
    let state=AppState{scheduler:Arc::new(AxScheduler::new()),database,api_token};
    let listener=tokio::net::TcpListener::bind(address).await?;
    println!("APOLO AX API listening on {address}");
    axum::serve(listener,app(state)).with_graceful_shutdown(async{let _=tokio::signal::ctrl_c().await;}).await?;
    Ok(())
}
#[cfg(test)]
mod tests{
    use super::*;
    use axum::{body::{to_bytes,Body},http::Request};
    use tower::ServiceExt;
    fn test_state()->AppState{AppState{scheduler:Arc::new(AxScheduler::new()),database:None,api_token:None}}
    #[tokio::test] async fn health_endpoint_reports_service_and_database_state(){
        let response=app(test_state()).oneshot(Request::builder().uri("/healthz").body(Body::empty()).unwrap()).await.unwrap();
        assert_eq!(response.status(),StatusCode::OK);
        let body=to_bytes(response.into_body(),4096).await.unwrap();let json:serde_json::Value=serde_json::from_slice(&body).unwrap();
        assert_eq!(json["service"],"apolo-ax-api");assert_eq!(json["status"],"ok");assert_eq!(json["database"],"not_configured");
    }
    #[tokio::test] async fn overview_does_not_invent_physical_nodes(){
        let response=app(test_state()).oneshot(Request::builder().uri("/api/v1/overview").body(Body::empty()).unwrap()).await.unwrap();
        assert_eq!(response.status(),StatusCode::OK);let body=to_bytes(response.into_body(),4096).await.unwrap();
        let json:serde_json::Value=serde_json::from_slice(&body).unwrap();assert_eq!(json["node_count"],0);assert_eq!(json["persistence"],"volatile_memory");
    }
    #[tokio::test] async fn projects_reject_missing_bearer_token(){
        let state=AppState{scheduler:Arc::new(AxScheduler::new()),database:None,api_token:Some("0123456789abcdef0123456789abcdef".into())};
        let response=app(state).oneshot(Request::builder().uri("/api/v1/projects").body(Body::empty()).unwrap()).await.unwrap();
        assert_eq!(response.status(),StatusCode::UNAUTHORIZED);
    }
    #[tokio::test] async fn projects_reject_missing_bearer_token_when_database_is_unconfigured(){
        let response=app(test_state()).oneshot(Request::builder().uri("/api/v1/projects").body(Body::empty()).unwrap()).await.unwrap();
        assert_eq!(response.status(),StatusCode::UNAUTHORIZED);
    }
}

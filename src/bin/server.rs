use axum::{
    Json, Router,
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    routing::get,
};
use rusqlite::{Connection, params};
use serde::Deserialize;
use serde_json::{Value, json};
use std::{
    env,
    sync::{Arc, Mutex},
};
use tower_http::services::ServeDir;

type App = Arc<Store>;
struct Store {
    db: Mutex<Connection>,
    token: String,
}
type Error = (StatusCode, String);
fn internal(e: impl std::fmt::Display) -> Error {
    eprintln!("{e}");
    (StatusCode::INTERNAL_SERVER_ERROR, "Internal error".into())
}
fn bad(s: &str) -> Error {
    (StatusCode::BAD_REQUEST, s.into())
}
fn authorize(s: &Store, h: &HeaderMap) -> Result<(), Error> {
    if h.get("authorization").and_then(|v| v.to_str().ok()) == Some(&format!("Bearer {}", s.token))
    {
        Ok(())
    } else {
        Err((StatusCode::UNAUTHORIZED, "Invalid token".into()))
    }
}
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let token = env::var("SPARESWARM_TOKEN").expect("Set SPARESWARM_TOKEN to a strong API token");
    assert!(
        token.len() >= 24,
        "SPARESWARM_TOKEN must contain at least 24 characters"
    );
    let db = Connection::open(env::var("DATABASE_PATH").unwrap_or("spareswarm.db".into()))?;
    db.execute_batch("PRAGMA foreign_keys=ON; PRAGMA journal_mode=WAL; PRAGMA busy_timeout=5000;
        CREATE TABLE IF NOT EXISTS projects(id TEXT PRIMARY KEY,name TEXT NOT NULL,repo TEXT NOT NULL,description TEXT NOT NULL);
        CREATE TABLE IF NOT EXISTS tasks(id TEXT PRIMARY KEY,project_id TEXT NOT NULL REFERENCES projects(id),title TEXT NOT NULL,description TEXT NOT NULL,status TEXT NOT NULL DEFAULT 'open' CHECK(status IN ('open','working','done')),created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP);
        CREATE TABLE IF NOT EXISTS updates(id TEXT PRIMARY KEY,task_id TEXT NOT NULL REFERENCES tasks(id),content TEXT NOT NULL,link TEXT,created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP);")?;
    let state = Arc::new(Store {
        db: Mutex::new(db),
        token,
    });
    let app = Router::new()
        .route("/health", get(|| async { "ok" }))
        .route("/api/projects", get(projects))
        .route("/api/tasks", get(tasks).post(create))
        .route("/api/tasks/{id}", get(detail).patch(update))
        .fallback_service(ServeDir::new(env::var("WEB_DIR").unwrap_or("web".into())))
        .layer(axum::extract::DefaultBodyLimit::max(64 * 1024))
        .with_state(state);
    let addr = env::var("LISTEN_ADDR").unwrap_or("127.0.0.1:8080".into());
    println!("SpareSwarm listening on {addr}");
    axum::serve(tokio::net::TcpListener::bind(addr).await?, app)
        .with_graceful_shutdown(async {
            let _ = tokio::signal::ctrl_c().await;
        })
        .await?;
    Ok(())
}
async fn projects(State(s): State<App>) -> Result<Json<Vec<Value>>, Error> {
    let db = s.db.lock().map_err(internal)?;
    let mut q = db
        .prepare("SELECT id,name,repo,description FROM projects ORDER BY name")
        .map_err(internal)?;
    let rows=q.query_map([],|r|Ok(json!({"id":r.get::<_,String>(0)?,"name":r.get::<_,String>(1)?,"repo":r.get::<_,String>(2)?,"description":r.get::<_,String>(3)?}))).map_err(internal)?;
    Ok(Json(rows.collect::<Result<Vec<_>, _>>().map_err(internal)?))
}
#[derive(Deserialize)]
struct Filter {
    project: Option<String>,
    status: Option<String>,
}
async fn tasks(State(s): State<App>, Query(f): Query<Filter>) -> Result<Json<Vec<Value>>, Error> {
    let db = s.db.lock().map_err(internal)?;
    let mut q=db.prepare("SELECT id,project_id,title,description,status,created_at,updated_at FROM tasks WHERE (?1 IS NULL OR project_id=?1) AND (?2 IS NULL OR status=?2) ORDER BY created_at DESC,id LIMIT 500").map_err(internal)?;
    let rows = q
        .query_map(params![f.project, f.status], task_row)
        .map_err(internal)?;
    Ok(Json(rows.collect::<Result<Vec<_>, _>>().map_err(internal)?))
}
fn task_row(r: &rusqlite::Row) -> rusqlite::Result<Value> {
    Ok(
        json!({"id":r.get::<_,String>(0)?,"project_id":r.get::<_,String>(1)?,"title":r.get::<_,String>(2)?,"description":r.get::<_,String>(3)?,"status":r.get::<_,String>(4)?,"created_at":r.get::<_,String>(5)?,"updated_at":r.get::<_,String>(6)?}),
    )
}
async fn detail(State(s): State<App>, Path(id): Path<String>) -> Result<Json<Value>, Error> {
    let db = s.db.lock().map_err(internal)?;
    Ok(Json(read_task(&db, &id)?))
}
fn read_task(db: &Connection, id: &str) -> Result<Value, Error> {
    let mut task=db.query_row("SELECT id,project_id,title,description,status,created_at,updated_at FROM tasks WHERE id=?",[id],task_row).map_err(|e|match e {rusqlite::Error::QueryReturnedNoRows=>(StatusCode::NOT_FOUND,"Task not found".into()),other=>internal(other)})?;
    let mut q = db
        .prepare("SELECT id,content,link,created_at FROM updates WHERE task_id=? ORDER BY rowid")
        .map_err(internal)?;
    let rows=q.query_map([id],|r|Ok(json!({"id":r.get::<_,String>(0)?,"content":r.get::<_,String>(1)?,"link":r.get::<_,Option<String>>(2)?,"created_at":r.get::<_,String>(3)?}))).map_err(internal)?;
    task["updates"] = json!(rows.collect::<Result<Vec<_>, _>>().map_err(internal)?);
    Ok(task)
}
#[derive(Deserialize)]
struct NewTask {
    project_id: String,
    title: String,
    #[serde(default)]
    description: String,
}
async fn create(
    State(s): State<App>,
    h: HeaderMap,
    Json(t): Json<NewTask>,
) -> Result<(StatusCode, Json<Value>), Error> {
    authorize(&s, &h)?;
    if t.title.trim().is_empty() || t.title.len() > 200 || t.description.len() > 20000 {
        return Err(bad("Title required (max 200); description max 20000"));
    }
    let db = s.db.lock().map_err(internal)?;
    let exists: bool = db
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM projects WHERE id=?)",
            [&t.project_id],
            |r| r.get(0),
        )
        .map_err(internal)?;
    if !exists {
        return Err(bad("Unknown project"));
    }
    let id = uuid::Uuid::new_v4().to_string();
    db.execute(
        "INSERT INTO tasks(id,project_id,title,description) VALUES(?,?,?,?)",
        params![id, t.project_id, t.title.trim(), t.description],
    )
    .map_err(internal)?;
    Ok((StatusCode::CREATED, Json(read_task(&db, &id)?)))
}
#[derive(Deserialize)]
struct Change {
    status: Option<String>,
    content: Option<String>,
    link: Option<String>,
}
async fn update(
    State(s): State<App>,
    h: HeaderMap,
    Path(id): Path<String>,
    Json(t): Json<Change>,
) -> Result<Json<Value>, Error> {
    authorize(&s, &h)?;
    if t.status.is_none() && t.content.as_deref().is_none_or(|x| x.trim().is_empty()) {
        return Err(bad("Provide status or content"));
    }
    if t.status
        .as_deref()
        .is_some_and(|x| !matches!(x, "open" | "working" | "done"))
    {
        return Err(bad("Invalid status"));
    }
    if t.content.as_ref().is_some_and(|x| x.len() > 20000) {
        return Err(bad("Content too long"));
    }
    if t.link
        .as_ref()
        .is_some_and(|x| x.len() > 2048 || !(x.starts_with("https://") || x.starts_with("http://")))
    {
        return Err(bad("Link must be HTTP(S), max 2048 characters"));
    }
    if t.link.is_some() && t.content.as_deref().is_none_or(|x| x.trim().is_empty()) {
        return Err(bad("A link requires content"));
    }
    let mut db = s.db.lock().map_err(internal)?;
    let tx = db.transaction().map_err(internal)?;
    read_task(&tx, &id)?;
    tx.execute(
        "UPDATE tasks SET status=COALESCE(?1,status),updated_at=CURRENT_TIMESTAMP WHERE id=?2",
        params![t.status, id],
    )
    .map_err(internal)?;
    if let Some(content) = t.content.filter(|x| !x.trim().is_empty()) {
        tx.execute(
            "INSERT INTO updates(id,task_id,content,link) VALUES(?,?,?,?)",
            params![uuid::Uuid::new_v4().to_string(), id, content, t.link],
        )
        .map_err(internal)?;
    }
    tx.commit().map_err(internal)?;
    Ok(Json(read_task(&db, &id)?))
}

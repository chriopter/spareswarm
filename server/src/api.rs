use crate::auth;
use axum::{
    Json,
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
};
use rusqlite::{Connection, params};
use serde::Deserialize;
use serde_json::{Value, json};
use std::sync::{Arc, Mutex};
pub(crate) type App = Arc<Store>;
pub(crate) struct Store {
    pub db: Mutex<Connection>,
    pub admin_hash: String,
}
pub(crate) type Error = (StatusCode, String);
pub(crate) fn internal(e: impl std::fmt::Display) -> Error {
    eprintln!("{e}");
    (StatusCode::INTERNAL_SERVER_ERROR, "Internal error".into())
}
fn bad(s: &str) -> Error {
    (StatusCode::BAD_REQUEST, s.into())
}
const TASK_SELECT: &str = "SELECT id,project_id,title,description,status,created_at,updated_at,author_id,help_reason FROM tasks";
pub async fn projects(State(s): State<App>) -> Result<Json<Vec<Value>>, Error> {
    let db = s.db.lock().map_err(internal)?;
    let mut q = db
        .prepare(
            "SELECT id,name,repo,description,summary,archived,category FROM projects ORDER BY name",
        )
        .map_err(internal)?;
    let rows=q.query_map([],|r|Ok(json!({"id":r.get::<_,String>(0)?,"name":r.get::<_,String>(1)?,"repo":r.get::<_,String>(2)?,"description":r.get::<_,String>(3)?,"summary":r.get::<_,String>(4)?,"archived":r.get::<_,bool>(5)?,"category":r.get::<_,String>(6)?}))).map_err(internal)?;
    Ok(Json(rows.collect::<Result<Vec<_>, _>>().map_err(internal)?))
}
#[derive(Deserialize)]
pub struct Filter {
    project: Option<String>,
    status: Option<String>,
    #[serde(default)]
    mine: bool,
    limit: Option<u32>,
    offset: Option<u32>,
}
pub async fn tasks(
    State(s): State<App>,
    headers: HeaderMap,
    Query(f): Query<Filter>,
) -> Result<Json<Vec<Value>>, Error> {
    let db = s.db.lock().map_err(internal)?;
    let author = if f.mine {
        Some(auth::participant(&db, &headers)?.id)
    } else {
        None
    };
    let mut q=db.prepare(&format!("{TASK_SELECT} WHERE (?1 IS NULL OR project_id=?1) AND (?2 IS NULL OR status=?2) AND (?3 IS NULL OR author_id=?3) ORDER BY created_at DESC,id LIMIT ?4 OFFSET ?5")).map_err(internal)?;
    let rows = q
        .query_map(
            params![
                f.project,
                f.status,
                author,
                f.limit.unwrap_or(500).clamp(1, 500),
                f.offset.unwrap_or(0)
            ],
            task_row,
        )
        .map_err(internal)?;
    let mut tasks = rows.collect::<Result<Vec<_>, _>>().map_err(internal)?;
    for task in &mut tasks {
        add_author(&db, task)?;
    }
    Ok(Json(tasks))
}
fn add_author(db: &Connection, record: &mut Value) -> Result<(), Error> {
    record["author"] = if let Some(id) = record["author_id"].as_str() {
        auth::profile(db, id)?
    } else {
        Value::Null
    };
    Ok(())
}
fn task_row(r: &rusqlite::Row) -> rusqlite::Result<Value> {
    Ok(
        json!({"id":r.get::<_,String>(0)?,"project_id":r.get::<_,String>(1)?,"title":r.get::<_,String>(2)?,"description":r.get::<_,String>(3)?,"status":r.get::<_,String>(4)?,"created_at":r.get::<_,String>(5)?,"updated_at":r.get::<_,String>(6)?,"author_id":r.get::<_,Option<String>>(7)?,"help_reason":r.get::<_,Option<String>>(8)?}),
    )
}
fn read_task(db: &Connection, id: &str) -> Result<Value, Error> {
    let mut task = db
        .query_row(&format!("{TASK_SELECT} WHERE id=?"), [id], task_row)
        .map_err(|e| match e {
            rusqlite::Error::QueryReturnedNoRows => {
                (StatusCode::NOT_FOUND, "Task not found".into())
            }
            other => internal(other),
        })?;
    let mut q=db.prepare("SELECT id,task_id,author_id,content,link,created_at,reply_to FROM comments WHERE task_id=? ORDER BY rowid").map_err(internal)?;
    let rows=q.query_map([id],|r|Ok(json!({"id":r.get::<_,String>(0)?,"task_id":r.get::<_,String>(1)?,"author_id":r.get::<_,Option<String>>(2)?,"content":r.get::<_,String>(3)?,"link":r.get::<_,Option<String>>(4)?,"created_at":r.get::<_,String>(5)?,"reply_to":r.get::<_,Option<String>>(6)?}))).map_err(internal)?;
    let mut comments = rows.collect::<Result<Vec<_>, _>>().map_err(internal)?;
    for c in &mut comments {
        add_author(db, c)?;
    }
    task["comments"] = json!(comments);
    add_author(db, &mut task)?;
    Ok(task)
}
pub async fn detail(State(s): State<App>, Path(id): Path<String>) -> Result<Json<Value>, Error> {
    let db = s.db.lock().map_err(internal)?;
    Ok(Json(read_task(&db, &id)?))
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NewTask {
    project_id: String,
    title: String,
    #[serde(default)]
    description: String,
}
pub async fn create(
    State(s): State<App>,
    headers: HeaderMap,
    Json(t): Json<NewTask>,
) -> Result<(StatusCode, Json<Value>), Error> {
    let db = s.db.lock().map_err(internal)?;
    let who = auth::participant(&db, &headers)?;
    if t.title.trim().is_empty() || t.title.len() > 200 || t.description.len() > 20000 {
        return Err(bad("Title required (max 200); description max 20000"));
    }
    let exists: bool = db
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM projects WHERE id=? AND archived=0)",
            [&t.project_id],
            |r| r.get(0),
        )
        .map_err(internal)?;
    if !exists {
        return Err(bad("Unknown or archived project"));
    }
    let id = uuid::Uuid::new_v4().to_string();
    db.execute(
        "INSERT INTO tasks(id,project_id,author_id,title,description) VALUES(?,?,?,?,?)",
        params![id, t.project_id, who.id, t.title.trim(), t.description],
    )
    .map_err(internal)?;
    Ok((StatusCode::CREATED, Json(read_task(&db, &id)?)))
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Change {
    status: Option<String>,
    help_reason: Option<String>,
    title: Option<String>,
    description: Option<String>,
}
pub async fn update(
    State(s): State<App>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(t): Json<Change>,
) -> Result<Json<Value>, Error> {
    let db = s.db.lock().map_err(internal)?;
    let who = auth::participant(&db, &headers)?;
    let task = read_task(&db, &id)?;
    if task["author_id"].as_str() != Some(&who.id) {
        return Err((
            StatusCode::FORBIDDEN,
            "Only the task author may edit this task".into(),
        ));
    }
    if t.status.is_none() && t.title.is_none() && t.description.is_none() && t.help_reason.is_none()
    {
        return Err(bad("No changes provided"));
    }
    if t.status
        .as_deref()
        .is_some_and(|s| !matches!(s, "open" | "working" | "needs_help" | "done"))
    {
        return Err(bad("Invalid status"));
    }
    if t.title
        .as_ref()
        .is_some_and(|s| s.trim().is_empty() || s.len() > 200)
        || t.description.as_ref().is_some_and(|s| s.len() > 20000)
    {
        return Err(bad("Invalid title or description"));
    }
    let next_status = t
        .status
        .as_deref()
        .unwrap_or(task["status"].as_str().unwrap());
    if next_status != "needs_help" && t.help_reason.as_ref().is_some_and(|s| !s.is_empty()) {
        return Err(bad("help_reason requires needs_help status"));
    }
    let help_reason = if next_status == "needs_help" {
        let reason = t
            .help_reason
            .as_deref()
            .or(task["help_reason"].as_str())
            .unwrap_or("")
            .trim();
        if reason.is_empty() || reason.len() > 2000 {
            return Err(bad("needs_help requires help_reason (1–2000 characters)"));
        }
        Some(reason)
    } else {
        None
    };
    db.execute("UPDATE tasks SET status=?1,title=COALESCE(?2,title),description=COALESCE(?3,description),help_reason=?4,updated_at=CURRENT_TIMESTAMP WHERE id=?5",params![next_status,t.title.as_deref().map(str::trim),t.description,help_reason,id]).map_err(internal)?;
    Ok(Json(read_task(&db, &id)?))
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Comment {
    content: String,
    link: Option<String>,
    reply_to: Option<String>,
}
pub async fn comment(
    State(s): State<App>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(t): Json<Comment>,
) -> Result<(StatusCode, Json<Value>), Error> {
    let db = s.db.lock().map_err(internal)?;
    let who = auth::participant(&db, &headers)?;
    read_task(&db, &id)?;
    if t.content.trim().is_empty() || t.content.len() > 20000 {
        return Err(bad("Comment required (max 20000)"));
    }
    if t.link
        .as_ref()
        .is_some_and(|s| s.len() > 2048 || !(s.starts_with("https://") || s.starts_with("http://")))
    {
        return Err(bad("Link must be HTTP(S), max 2048 characters"));
    }
    if let Some(parent) = &t.reply_to {
        let valid: bool = db
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM comments WHERE id=? AND task_id=?)",
                params![parent, id],
                |r| r.get(0),
            )
            .map_err(internal)?;
        if !valid {
            return Err(bad("Reply must reference a comment on this task"));
        }
    }
    let cid = uuid::Uuid::new_v4().to_string();
    db.execute(
        "INSERT INTO comments(id,task_id,author_id,content,link,reply_to) VALUES(?,?,?,?,?,?)",
        params![cid, id, who.id, t.content, t.link, t.reply_to],
    )
    .map_err(internal)?;
    Ok((
        StatusCode::CREATED,
        Json(json!({"id":cid,"task_id":id,"author_id":who.id,"reply_to":t.reply_to})),
    ))
}
pub async fn health(State(s): State<App>) -> Result<&'static str, Error> {
    s.db.lock()
        .map_err(internal)?
        .query_row("SELECT 1", [], |_| Ok(()))
        .map_err(internal)?;
    Ok("ok")
}

pub async fn home(State(s): State<App>, headers: HeaderMap) -> Result<Json<Value>, Error> {
    let db = s.db.lock().map_err(internal)?;
    let who = auth::participant(&db, &headers)?;
    let summary = |sql: &str, own: bool| -> Result<Vec<Value>, Error> {
        let mut q = db.prepare(sql).map_err(internal)?;
        let row = |r: &rusqlite::Row| {
            Ok(json!({
                "id":r.get::<_,String>(0)?,"project_id":r.get::<_,String>(1)?,
                "title":r.get::<_,String>(2)?,"status":r.get::<_,String>(3)?,
                "description_preview":r.get::<_,String>(4)?,"updated_at":r.get::<_,String>(5)?,"help_reason":r.get::<_,Option<String>>(6)?
            }))
        };
        let rows = if own {
            q.query_map([&who.id], row)
        } else {
            q.query_map([], row)
        }
        .map_err(internal)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(internal)
    };
    let active = summary(
        "SELECT id,project_id,title,status,substr(description,1,240),updated_at,help_reason FROM tasks WHERE author_id=? AND status='working' ORDER BY updated_at DESC,id LIMIT 10",
        true,
    )?;
    let open = summary(
        "SELECT id,project_id,title,status,substr(description,1,240),updated_at,help_reason FROM tasks WHERE status='open' AND project_id IN (SELECT id FROM projects WHERE archived=0) ORDER BY created_at DESC,id LIMIT 10",
        false,
    )?;
    let help = summary(
        "SELECT id,project_id,title,status,substr(description,1,240),updated_at,help_reason FROM tasks WHERE status='needs_help' ORDER BY updated_at DESC,id LIMIT 10",
        false,
    )?;
    let mut q=db.prepare("SELECT c.id,c.task_id,c.author_id,substr(c.content,1,320),c.reply_to,c.created_at,t.title FROM comments c JOIN tasks t ON t.id=c.task_id LEFT JOIN comments parent ON parent.id=c.reply_to WHERE c.author_id IS NOT NULL AND c.author_id<>?1 AND (t.author_id=?1 OR parent.author_id=?1) ORDER BY c.rowid DESC LIMIT 10").map_err(internal)?;
    let comments=q.query_map([&who.id],|r|Ok(json!({"id":r.get::<_,String>(0)?,"task_id":r.get::<_,String>(1)?,"author_id":r.get::<_,String>(2)?,"content_preview":r.get::<_,String>(3)?,"reply_to":r.get::<_,Option<String>>(4)?,"created_at":r.get::<_,String>(5)?,"task_title":r.get::<_,String>(6)?}))).map_err(internal)?.collect::<Result<Vec<_>,_>>().map_err(internal)?;
    Ok(Json(
        json!({"participant":auth::profile(&db,&who.id)?,"working_tasks":active,"needs_help_tasks":help,"recent_responses":comments,"open_tasks":open,"limit_per_section":10}),
    ))
}

pub async fn activity(State(s): State<App>) -> Result<Json<Value>, Error> {
    let db = s.db.lock().map_err(internal)?;
    let mut q=db.prepare("SELECT p.id,p.name,p.model,p.last_seen,COALESCE((SELECT id FROM tasks WHERE author_id=p.id AND status IN ('working','needs_help') ORDER BY updated_at DESC LIMIT 1),(SELECT task_id FROM comments WHERE author_id=p.id AND created_at>=datetime('now','-3600 seconds') ORDER BY rowid DESC LIMIT 1)) FROM participants p WHERE last_seen>=datetime('now','-3600 seconds') ORDER BY last_seen DESC,p.id LIMIT 50").map_err(internal)?;
    let rows=q.query_map([],|r|Ok((json!({"id":r.get::<_,String>(0)?,"name":r.get::<_,Option<String>>(1)?,"model":r.get::<_,Option<String>>(2)?,"last_seen":r.get::<_,String>(3)?}),r.get::<_,Option<String>>(4)?))).map_err(internal)?.collect::<Result<Vec<_>,_>>().map_err(internal)?;
    let mut agents = Vec::new();
    for (mut agent, id) in rows {
        agent["task"] = if let Some(id) = id {
            let task = read_task(&db, &id)?;
            json!({"id":task["id"],"title":task["title"],"project_id":task["project_id"],"status":task["status"]})
        } else {
            Value::Null
        };
        agents.push(agent);
    }
    let online_count: i64 = db
        .query_row(
            "SELECT count(*) FROM participants WHERE last_seen>=datetime('now','-600 seconds')",
            [],
            |r| r.get(0),
        )
        .map_err(internal)?;
    Ok(Json(
        json!({"agents":agents,"window_seconds":3600,"online_count":online_count}),
    ))
}

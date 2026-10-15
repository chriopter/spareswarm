use crate::api::{App, Error, internal};
use axum::{
    Json,
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
};
use rusqlite::{Connection, OptionalExtension, params};
use serde::Deserialize;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

pub fn hash(token: &str) -> String {
    Sha256::digest(token.as_bytes())
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}
fn bearer(headers: &HeaderMap) -> Result<&str, Error> {
    headers
        .get("authorization")
        .and_then(|h| h.to_str().ok())
        .and_then(|h| h.strip_prefix("Bearer "))
        .ok_or((
            StatusCode::UNAUTHORIZED,
            "Participant token required".into(),
        ))
}
pub fn admin(headers: &HeaderMap, expected: &str) -> Result<(), Error> {
    let actual = hash(bearer(headers)?);
    let mismatch = actual
        .bytes()
        .zip(expected.bytes())
        .fold(0u8, |a, (b, c)| a | (b ^ c));
    if mismatch == 0 {
        Ok(())
    } else {
        Err((StatusCode::UNAUTHORIZED, "Operator token required".into()))
    }
}
pub struct Identity {
    pub id: String,
    pub token_id: String,
}
pub fn participant(db: &Connection, headers: &HeaderMap) -> Result<Identity, Error> {
    let who = db
        .query_row(
            "SELECT participant_id,id FROM tokens WHERE token_hash=? AND revoked_at IS NULL",
            [hash(bearer(headers)?)],
            |r| {
                Ok(Identity {
                    id: r.get(0)?,
                    token_id: r.get(1)?,
                })
            },
        )
        .optional()
        .map_err(internal)?
        .ok_or((
            StatusCode::UNAUTHORIZED,
            "Invalid or revoked participant token".into(),
        ))?;
    db.execute("UPDATE participants SET last_seen=CURRENT_TIMESTAMP WHERE id=? AND (last_seen IS NULL OR last_seen<datetime('now','-10 seconds'))",[&who.id]).map_err(internal)?;
    Ok(who)
}

fn issue(db: &Connection, id: &str) -> Result<Value, Error> {
    let token = format!(
        "ss_{}{}",
        uuid::Uuid::new_v4().simple(),
        uuid::Uuid::new_v4().simple()
    );
    let token_id = uuid::Uuid::new_v4().to_string();
    db.execute(
        "INSERT INTO tokens(id,participant_id,token_hash) VALUES(?,?,?)",
        params![token_id, id, hash(&token)],
    )
    .map_err(internal)?;
    Ok(json!({"participant_id":id,"token_id":token_id,"token":token}))
}
pub async fn register(
    State(s): State<App>,
    headers: HeaderMap,
) -> Result<(StatusCode, Json<Value>), Error> {
    admin(&headers, &s.admin_hash)?;
    let mut db = s.db.lock().map_err(internal)?;
    let tx = db.transaction().map_err(internal)?;
    let id = uuid::Uuid::new_v4().to_string();
    tx.execute("INSERT INTO participants(id) VALUES(?)", [&id])
        .map_err(internal)?;
    let result = issue(&tx, &id)?;
    tx.commit().map_err(internal)?;
    Ok((StatusCode::CREATED, Json(result)))
}
pub async fn me(State(s): State<App>, headers: HeaderMap) -> Result<Json<Value>, Error> {
    let db = s.db.lock().map_err(internal)?;
    let who = participant(&db, &headers)?;
    let mut profile = profile(&db, &who.id)?;
    profile["participant_id"] = json!(who.id);
    profile["token_id"] = json!(who.token_id);
    Ok(Json(profile))
}
pub async fn new_token(
    State(s): State<App>,
    headers: HeaderMap,
) -> Result<(StatusCode, Json<Value>), Error> {
    let db = s.db.lock().map_err(internal)?;
    let who = participant(&db, &headers)?;
    Ok((StatusCode::CREATED, Json(issue(&db, &who.id)?)))
}
pub async fn revoke(
    State(s): State<App>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<StatusCode, Error> {
    let db = s.db.lock().map_err(internal)?;
    if admin(&headers, &s.admin_hash).is_ok() {
        if db
            .execute(
                "UPDATE tokens SET revoked_at=COALESCE(revoked_at,CURRENT_TIMESTAMP) WHERE id=?",
                [id],
            )
            .map_err(internal)?
            == 0
        {
            return Err((StatusCode::NOT_FOUND, "Token not found".into()));
        }
    } else {
        let who = participant(&db, &headers)?;
        if db.execute("UPDATE tokens SET revoked_at=COALESCE(revoked_at,CURRENT_TIMESTAMP) WHERE id=? AND participant_id=?",params![id,who.id]).map_err(internal)?==0{return Err((StatusCode::NOT_FOUND,"Token not found".into()))}
    }
    Ok(StatusCode::NO_CONTENT)
}

pub fn profile(db: &Connection, id: &str) -> Result<Value, Error> {
    db.query_row("SELECT id,name,model FROM participants WHERE id=?",[id],|r|Ok(json!({"id":r.get::<_,String>(0)?,"name":r.get::<_,Option<String>>(1)?,"model":r.get::<_,Option<String>>(2)?}))).map_err(internal)
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProfileChange {
    pub name: Option<String>,
    pub model: Option<String>,
}
pub async fn edit_profile(
    State(s): State<App>,
    headers: HeaderMap,
    Json(change): Json<ProfileChange>,
) -> Result<Json<Value>, Error> {
    if change.name.as_ref().is_some_and(|s| s.len() > 100)
        || change.model.as_ref().is_some_and(|s| s.len() > 100)
    {
        return Err((
            StatusCode::BAD_REQUEST,
            "Name and model: max 100 characters".into(),
        ));
    }
    let db = s.db.lock().map_err(internal)?;
    let who = participant(&db, &headers)?;
    db.execute(
        "UPDATE participants SET name=COALESCE(?1,name),model=COALESCE(?2,model) WHERE id=?3",
        params![change.name, change.model, who.id],
    )
    .map_err(internal)?;
    Ok(Json(profile(&db, &who.id)?))
}

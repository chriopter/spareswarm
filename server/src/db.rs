use rusqlite::Connection;
pub fn open(path: &str) -> rusqlite::Result<Connection> {
    let mut db = Connection::open(path)?;
    db.execute_batch("PRAGMA foreign_keys=ON; PRAGMA journal_mode=WAL; PRAGMA busy_timeout=5000;")?;
    let version: i64 = db.query_row("PRAGMA user_version", [], |r| r.get(0))?;
    if version > 8 {
        return Err(rusqlite::Error::InvalidQuery);
    }
    if version < 2 {
        let tx = db.transaction()?;
        tx.execute_batch(include_str!("../migrations/001_initial.sql"))?;
        tx.execute_batch(include_str!("../migrations/002_participants.sql"))?;
        tx.commit()?;
    }
    if version < 3 {
        db.execute_batch(include_str!("../migrations/003_comment_replies.sql"))?;
    }
    if version < 4 {
        db.execute_batch(include_str!("../migrations/004_project_summary.sql"))?;
    }
    if version < 5 {
        db.execute_batch(include_str!("../migrations/005_presence.sql"))?;
    }
    if version < 6 {
        db.pragma_update(None, "foreign_keys", false)?;
        let tx = db.transaction()?;
        tx.execute_batch(include_str!("../migrations/006_needs_help.sql"))?;
        tx.commit()?;
        db.pragma_update(None, "foreign_keys", true)?;
        let violations: i64 =
            db.query_row("SELECT count(*) FROM pragma_foreign_key_check", [], |r| {
                r.get(0)
            })?;
        if violations != 0 {
            return Err(rusqlite::Error::InvalidQuery);
        }
    }
    if version < 7 {
        db.execute_batch(include_str!("../migrations/007_project_catalog.sql"))?;
    }
    if version < 8 {
        db.execute_batch(include_str!("../migrations/008_project_category.sql"))?;
    }
    Ok(db)
}

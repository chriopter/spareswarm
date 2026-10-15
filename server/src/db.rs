use rusqlite::Connection;
pub fn open(path: &str) -> rusqlite::Result<Connection> {
    let db = Connection::open(path)?;
    db.execute_batch(include_str!("../migrations/001_initial.sql"))?;
    Ok(db)
}

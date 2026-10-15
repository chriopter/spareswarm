use rusqlite::{Connection, params};
use serde::Deserialize;
use std::{collections::HashSet, error::Error, fs, path::Path};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Project {
    id: String,
    name: String,
    repo: String,
    summary: String,
    description: String,
}

/// Validate the entire catalog before changing any stored project.
pub fn sync(db: &mut Connection, directory: &Path) -> Result<(), Box<dyn Error>> {
    let mut projects = Vec::new();
    let mut ids = HashSet::new();
    for entry in fs::read_dir(directory)? {
        let path = entry?.path();
        if path.extension().and_then(|s| s.to_str()) != Some("json") {
            continue;
        }
        let project: Project = serde_json::from_slice(&fs::read(&path)?)
            .map_err(|e| format!("{}: {e}", path.display()))?;
        let valid_id = !project.id.is_empty()
            && project.id.len() <= 100
            && project
                .id
                .bytes()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == b'-');
        if !valid_id
            || path.file_stem().and_then(|s| s.to_str()) != Some(&project.id)
            || !ids.insert(project.id.clone())
            || project.name.trim().is_empty()
            || project.name.len() > 200
            || project.summary.trim().is_empty()
            || project.summary.len() > 500
            || project.description.trim().is_empty()
            || project.description.len() > 20000
            || !project.repo.starts_with("https://github.com/")
            || project.repo.chars().any(char::is_whitespace)
        {
            return Err(format!("Invalid project definition: {}", path.display()).into());
        }
        projects.push(project);
    }
    let tx = db.transaction()?;
    tx.execute("UPDATE projects SET archived=1", [])?;
    for p in projects {
        tx.execute(
            "INSERT INTO projects(id,name,repo,summary,description,archived) VALUES(?1,?2,?3,?4,?5,0)
             ON CONFLICT(id) DO UPDATE SET name=excluded.name,repo=excluded.repo,
             summary=excluded.summary,description=excluded.description,archived=0",
            params![p.id, p.name, p.repo, p.summary, p.description],
        )?;
    }
    tx.commit()?;
    Ok(())
}

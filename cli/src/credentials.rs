use serde_json::{Map, Value, json};
use std::{env, fs, io::Write, path::PathBuf};
type Error = Box<dyn std::error::Error>;
fn path() -> Result<PathBuf, Error> {
    if let Some(p) = env::var_os("SPARESWARM_CONFIG") {
        return Ok(p.into());
    }
    let root = env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .unwrap_or(PathBuf::from(env::var_os("HOME").ok_or("HOME is missing")?).join(".config"));
    Ok(root.join("spareswarm/credentials.json"))
}
fn read() -> Result<Map<String, Value>, Error> {
    let p = path()?;
    if !p.exists() {
        return Ok(Map::new());
    }
    Ok(serde_json::from_slice(&fs::read(p)?)?)
}
pub fn load(url: &str) -> Result<Option<String>, Error> {
    Ok(read()?
        .get(url)
        .and_then(|v| v["token"].as_str())
        .map(str::to_owned))
}
pub fn save(url: &str, token: &str) -> Result<(), Error> {
    let mut values = read()?;
    values.insert(url.into(), json!({"token":token}));
    let p = path()?;
    let parent = p
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .ok_or("Config requires a parent directory")?;
    #[cfg(unix)]
    let parent_existed = parent.exists();
    fs::create_dir_all(parent)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if !parent_existed {
            fs::set_permissions(parent, fs::Permissions::from_mode(0o700))?;
        }
    }
    let temp = parent.join(format!(
        ".credentials-{}-{}.tmp",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_nanos()
    ));
    let mut options = fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut f = options.open(&temp)?;
    f.write_all(&serde_json::to_vec_pretty(&values)?)?;
    f.sync_all()?;
    fs::rename(temp, p)?;
    Ok(())
}

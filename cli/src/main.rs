mod credentials;
use clap::{Parser, Subcommand};
use serde_json::{Value, json};
use std::io::Read;
type Error = Box<dyn std::error::Error>;
#[derive(Parser)]
#[command(version, about = "Contribute agent time to open projects")]
struct Args {
    #[arg(long, env = "SPARESWARM_URL", default_value = "https://spareswarm.com")]
    url: String,
    #[arg(long, env = "SPARESWARM_TOKEN", hide_env_values = true)]
    token: Option<String>,
    #[command(subcommand)]
    command: Command,
}
#[derive(Subcommand)]
enum Command {
    Instructions,
    Home,
    Projects,
    Find {
        #[arg(long)]
        project: Option<String>,
        #[arg(long)]
        status: Option<String>,
        #[arg(long)]
        mine: bool,
    },
    Show {
        id: String,
    },
    Create {
        #[arg(long)]
        project: String,
        #[arg(long)]
        title: String,
        #[arg(long, default_value = "")]
        description: String,
    },
    Update {
        id: String,
        #[arg(long)]
        help_reason: Option<String>,
        #[arg(long)]
        title: Option<String>,
        #[arg(long)]
        description: Option<String>,
        #[arg(long,value_parser=["open","working","needs_help","done"])]
        status: Option<String>,
    },
    Comment {
        id: String,
        #[arg(long)]
        reply_to: Option<String>,
        #[arg(long)]
        text: String,
        #[arg(long)]
        link: Option<String>,
    },
    Profile {
        #[arg(long)]
        name: Option<String>,
        #[arg(long)]
        model: Option<String>,
    },
    Auth {
        #[command(subcommand)]
        command: Auth,
    },
}
#[derive(Subcommand)]
enum Auth {
    /// Create and save a participant identity; no operator token is needed.
    Register,
    Login,
    Whoami,
    Rotate,
    Revoke {
        token_id: String,
    },
}
async fn send(r: reqwest::RequestBuilder) -> Result<Value, Error> {
    let response = r.send().await?;
    let status = response.status();
    let body = response.text().await?;
    if !status.is_success() {
        return Err(format!("{status}: {body}").into());
    }
    if body.is_empty() {
        Ok(json!({"ok":true}))
    } else {
        Ok(serde_json::from_str(&body)?)
    }
}
fn required(token: &Option<String>) -> Result<&str, Error> {
    token
        .as_deref()
        .ok_or("Run `spareswarm auth login` or `auth register` first".into())
}
#[tokio::main]
async fn main() -> Result<(), Error> {
    let a = Args::parse();
    let url = a.url.trim_end_matches('/');
    let parsed = reqwest::Url::parse(url)?;
    if parsed.scheme() != "https"
        && !(parsed.scheme() == "http"
            && matches!(parsed.host_str(), Some("127.0.0.1" | "localhost" | "[::1]")))
    {
        return Err("Use HTTPS (HTTP is allowed only on localhost)".into());
    }
    let c = reqwest::Client::builder()
        .user_agent(concat!("SpareSwarm-CLI/", env!("CARGO_PKG_VERSION")))
        .redirect(reqwest::redirect::Policy::none())
        .build()?;
    let base = format!("{url}/api");
    let stored = credentials::load(url)?;
    let token = stored.or(a.token.clone());
    if matches!(a.command, Command::Instructions) {
        let response = c.get(format!("{url}/agent-instructions")).send().await?;
        let status = response.status();
        if !status.is_success() {
            return Err(format!("Instructions unavailable: {status}").into());
        }
        print!("{}", response.text().await?);
        return Ok(());
    }
    let value = match a.command {
        Command::Auth { command } => match command {
            Auth::Register => {
                if credentials::load(url)?.is_some() {
                    return Err("A participant is already saved for this server; use auth whoami or auth rotate".into());
                }
                let result = send(c.post(format!("{base}/participants"))).await?;
                credentials::save(url, result["token"].as_str().ok_or("Missing issued token")?)?;
                json!({"participant_id":result["participant_id"],"token_id":result["token_id"],"credentials_saved":true})
            }
            Auth::Login => {
                eprintln!("Paste participant token on stdin, then EOF:");
                let mut value = String::new();
                std::io::stdin().read_to_string(&mut value)?;
                let value = value.trim();
                let result = send(c.get(format!("{base}/me")).bearer_auth(value)).await?;
                credentials::save(url, value)?;
                result
            }
            Auth::Whoami => {
                send(c.get(format!("{base}/me")).bearer_auth(required(&token)?)).await?
            }
            Auth::Rotate => {
                let old = send(c.get(format!("{base}/me")).bearer_auth(required(&token)?)).await?;
                let result = send(
                    c.post(format!("{base}/tokens"))
                        .bearer_auth(required(&token)?),
                )
                .await?;
                let new = result["token"].as_str().ok_or("Missing issued token")?;
                credentials::save(url, new)?;
                send(
                    c.delete(format!(
                        "{base}/tokens/{}",
                        old["token_id"].as_str().ok_or("Missing token ID")?
                    ))
                    .bearer_auth(new),
                )
                .await?;
                json!({"participant_id":result["participant_id"],"token_id":result["token_id"],"rotated":true})
            }
            Auth::Revoke { token_id } => {
                send(
                    c.delete(format!("{base}/tokens/{token_id}"))
                        .bearer_auth(required(&token)?),
                )
                .await?
            }
        },
        Command::Profile { name, model } => {
            send(
                c.patch(format!("{base}/me"))
                    .bearer_auth(required(&token)?)
                    .json(&json!({"name":name,"model":model})),
            )
            .await?
        }
        Command::Instructions => unreachable!(),
        Command::Home => send(c.get(format!("{base}/home")).bearer_auth(required(&token)?)).await?,
        Command::Projects => send(c.get(format!("{base}/projects"))).await?,
        Command::Find {
            project,
            status,
            mine,
        } => {
            let mut r = c
                .get(format!("{base}/tasks"))
                .query(&[("mine", mine.to_string())]);
            if let Some(p) = project {
                r = r.query(&[("project", p)])
            }
            if let Some(s) = status {
                r = r.query(&[("status", s)])
            }
            if mine {
                r = r.bearer_auth(required(&token)?)
            }
            send(r).await?
        }
        Command::Show { id } => send(c.get(format!("{base}/tasks/{id}"))).await?,
        Command::Create {
            project,
            title,
            description,
        } => {
            send(
                c.post(format!("{base}/tasks"))
                    .bearer_auth(required(&token)?)
                    .json(&json!({"project_id":project,"title":title,"description":description})),
            )
            .await?
        }
        Command::Update {
            id,
            help_reason,
            title,
            description,
            status,
        } => {
            send(
                c.patch(format!("{base}/tasks/{id}"))
                    .bearer_auth(required(&token)?)
                    .json(&json!({"title":title,"description":description,"status":status,"help_reason":help_reason})),
            )
            .await?
        }
        Command::Comment {
            id,
            text,
            link,
            reply_to,
        } => {
            send(
                c.post(format!("{base}/tasks/{id}/comments"))
                    .bearer_auth(required(&token)?)
                    .json(&json!({"content":text,"link":link,"reply_to":reply_to})),
            )
            .await?
        }
    };
    println!("{}", serde_json::to_string_pretty(&value)?);
    Ok(())
}

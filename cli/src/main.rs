use clap::{Parser, Subcommand};
use serde_json::json;
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
    Projects,
    Find {
        #[arg(long)]
        project: Option<String>,
        #[arg(long)]
        status: Option<String>,
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
        text: Option<String>,
        #[arg(long,value_parser=["open","working","done"])]
        status: Option<String>,
        #[arg(long)]
        link: Option<String>,
    },
}
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let a = Args::parse();
    let c = reqwest::Client::new();
    let base = format!("{}/api", a.url.trim_end_matches('/'));
    let (request, write) = match a.command {
        Command::Projects => (c.get(format!("{base}/projects")), false),
        Command::Find { project, status } => {
            let mut q = vec![];
            if let Some(p) = project {
                q.push(("project", p));
            }
            if let Some(s) = status {
                q.push(("status", s));
            }
            (c.get(format!("{base}/tasks")).query(&q), false)
        }
        Command::Show { id } => (c.get(format!("{base}/tasks/{id}")), false),
        Command::Create {
            project,
            title,
            description,
        } => (
            c.post(format!("{base}/tasks"))
                .json(&json!({"project_id":project,"title":title,"description":description})),
            true,
        ),
        Command::Update {
            id,
            text,
            status,
            link,
        } => (
            c.patch(format!("{base}/tasks/{id}"))
                .json(&json!({"content":text,"status":status,"link":link})),
            true,
        ),
    };
    let request = if write {
        request.bearer_auth(a.token.ok_or("Set SPARESWARM_TOKEN for write access")?)
    } else {
        request
    };
    let response = request.send().await?;
    let status = response.status();
    let body = response.text().await?;
    if !status.is_success() {
        return Err(format!("{status}: {body}").into());
    }
    println!(
        "{}",
        serde_json::to_string_pretty(&serde_json::from_str::<serde_json::Value>(&body)?)?
    );
    Ok(())
}

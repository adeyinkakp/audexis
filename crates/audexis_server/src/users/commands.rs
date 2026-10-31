use super::{Result, quota};
use sha2::{Digest, Sha256};

pub enum Command {
    Help,
    Create { name: String, quota: Option<i64> },
    List,
    ResetKey(String),
    Quota { name: String, bytes: i64 },
    Disabled { name: String, disabled: bool },
}

pub fn parse(args: &[String]) -> Result<Option<Command>> {
    let command = match args
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>()
        .as_slice()
    {
        [] | ["serve"] => return Ok(None),
        ["--help" | "-h" | "help"] | ["users", "--help" | "-h" | "help"] | ["users"] => {
            Command::Help
        }
        ["users", "create", name] => Command::Create {
            name: username(name)?,
            quota: None,
        },
        ["users", "create", name, "--quota", size] => Command::Create {
            name: username(name)?,
            quota: Some(quota::parse(size)?),
        },
        ["users", "list"] => Command::List,
        ["users", "reset-key", name] => Command::ResetKey((*name).into()),
        ["users", "quota", name, size] => Command::Quota {
            name: (*name).into(),
            bytes: quota::parse(size)?,
        },
        ["users", "disable", name] => Command::Disabled {
            name: (*name).into(),
            disabled: true,
        },
        ["users", "enable", name] => Command::Disabled {
            name: (*name).into(),
            disabled: false,
        },
        _ => return Err("Unknown command or arguments. Run audexis_server --help.".into()),
    };
    Ok(Some(command))
}

fn username(name: &str) -> Result<String> {
    if name.is_empty()
        || name.len() > 64
        || !name
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b"._-".contains(&byte))
    {
        return Err("Use a name with 1–64 letters, numbers, dots, hyphens, or underscores.".into());
    }
    Ok(name.into())
}

fn new_key() -> (String, String) {
    let key = format!(
        "audexis_sh_{}{}",
        uuid::Uuid::new_v4().simple(),
        uuid::Uuid::new_v4().simple()
    );
    let hash = format!("{:x}", Sha256::digest(key.as_bytes()));
    (key, hash)
}

fn show_key(name: &str, key: &str) {
    println!(
        "User: {name}\nAPI key: {key}\n\nSave this key now; it cannot be displayed again.\nEnter it with your server URL in Audexis's custom-server settings."
    );
}

async fn user_id(pool: &sqlx::PgPool, name: &str) -> Result<String> {
    sqlx::query_scalar("SELECT user_id FROM self_hosted_users WHERE name=$1")
        .bind(name)
        .fetch_optional(pool)
        .await?
        .ok_or_else(|| "User not found. Run audexis_server users list.".into())
}

pub async fn run(pool: &sqlx::PgPool, command: Command) -> Result<()> {
    match command {
        Command::Help => super::help(),
        Command::Create {
            name,
            quota: requested,
        } => {
            let quota = match requested {
                Some(bytes) => bytes,
                None => quota::default_quota()?,
            };
            let id = format!("selfhosted:{}", uuid::Uuid::new_v4());
            let (key, hash) = new_key();
            let mut tx = pool.begin().await?;
            sqlx::query("INSERT INTO cloud_accounts(user_id,quota_bytes) VALUES ($1,$2)")
                .bind(&id)
                .bind(quota)
                .execute(&mut *tx)
                .await?;
            let created = sqlx::query("INSERT INTO self_hosted_users(user_id,name,api_key_sha256) VALUES ($1,$2,$3) ON CONFLICT(name) DO NOTHING")
                .bind(&id).bind(&name).bind(hash).execute(&mut *tx).await?;
            if created.rows_affected() == 0 {
                return Err(
                    "That username already exists. Use users reset-key to replace its key.".into(),
                );
            }
            tx.commit().await?;
            println!("Created account with a {quota}-byte quota.");
            show_key(&name, &key);
        }
        Command::List => {
            let rows: Vec<(String,bool,i64)> = sqlx::query_as("SELECT u.name,u.disabled,a.quota_bytes FROM self_hosted_users u JOIN cloud_accounts a ON a.user_id=u.user_id ORDER BY u.name")
                .fetch_all(pool).await?;
            if rows.is_empty() {
                println!("No self-hosted users. Run audexis_server users create <name>.");
            } else {
                println!("NAME\tSTATUS\tQUOTA (BYTES)");
                for (name, disabled, quota) in rows {
                    println!(
                        "{name}\t{}\t{quota}",
                        if disabled { "disabled" } else { "active" }
                    );
                }
            }
        }
        Command::ResetKey(name) => {
            let id = user_id(pool, &name).await?;
            let (key, hash) = new_key();
            sqlx::query("UPDATE self_hosted_users SET api_key_sha256=$1 WHERE user_id=$2")
                .bind(hash)
                .bind(id)
                .execute(pool)
                .await?;
            println!("Replaced API key. The previous key is no longer valid.");
            show_key(&name, &key);
        }
        Command::Quota { name, bytes } => {
            let id = user_id(pool, &name).await?;
            sqlx::query("UPDATE cloud_accounts SET quota_bytes=$1 WHERE user_id=$2")
                .bind(bytes)
                .bind(id)
                .execute(pool)
                .await?;
            println!("Quota for {name} is now {bytes} bytes. Existing files are kept.");
        }
        Command::Disabled { name, disabled } => {
            let id = user_id(pool, &name).await?;
            sqlx::query("UPDATE self_hosted_users SET disabled=$1 WHERE user_id=$2")
                .bind(disabled)
                .bind(id)
                .execute(pool)
                .await?;
            println!(
                "User {name} is now {}.",
                if disabled { "disabled" } else { "enabled" }
            );
        }
    }
    Ok(())
}

use anyhow::Result;
use dialect_coach_backend::persistence::{SledPersistence, UserPersistence};
use dialect_coach_shared::InviteCode;
use std::env;

#[tokio::main]
async fn main() -> Result<()> {
    let args: Vec<String> = env::args().collect();
    let db = SledPersistence::new("./data/sled.db")?;

    match args.get(1).map(String::as_str) {
        Some("generate-invite") => generate_invite(&db, &args).await,
        Some("list-invites") => list_invites(&db).await,
        Some("delete-invite") => delete_invite(&db, &args).await,
        _ => print_usage(),
    }
}

async fn generate_invite(db: &SledPersistence, args: &[String]) -> Result<()> {
    let expiration = parse_expiration(args)?;
    let code = generate_code();
    let invite = InviteCode::new(code.clone(), expiration);
    db.create_invite_code(&invite).await?;
    print_invite(&invite);
    Ok(())
}

async fn list_invites(db: &SledPersistence) -> Result<()> {
    let codes = db.list_invite_codes().await?;
    println!("Total invite codes: {}\n", codes.len());
    for invite in codes {
        print_invite(&invite);
    }
    Ok(())
}

async fn delete_invite(db: &SledPersistence, args: &[String]) -> Result<()> {
    let code = extract_code(args)?;
    db.delete_invite_code(&code).await?;
    println!("Deleted invite code: {}", code);
    Ok(())
}

fn extract_code(args: &[String]) -> Result<String> {
    args.get(2)
        .cloned()
        .ok_or_else(|| anyhow::anyhow!("delete-invite requires a code"))
}

fn parse_expiration(args: &[String]) -> Result<Option<i64>> {
    match args.iter().position(|a| a == "--expires-days") {
        Some(idx) => {
            let days: i64 = args.get(idx + 1)
                .ok_or_else(|| anyhow::anyhow!("--expires-days requires a number"))?
                .parse()?;
            Ok(Some(current_timestamp() + days * 86400))
        }
        None => Ok(None),
    }
}

fn generate_code() -> String {
    use rand::Rng;
    const CHARSET: &[u8] = b"ABCDEFGHJKLMNPQRSTUVWXYZ23456789";
    let mut rng = rand::thread_rng();
    (0..12).map(|_| CHARSET[rng.gen_range(0..CHARSET.len())] as char).collect()
}

fn print_invite(invite: &InviteCode) {
    println!("Code: {}", invite.code);
    println!("Created: {}", format_timestamp(invite.created_date));
    println!("Expires: {}", format_expiration(invite.expiration));
    println!("Status: {}", format_status(invite));
    println!();
}

fn format_timestamp(ts: i64) -> String {
    use chrono::{DateTime, Utc};
    DateTime::from_timestamp(ts, 0)
        .map(|dt: DateTime<Utc>| dt.format("%Y-%m-%d %H:%M:%S UTC").to_string())
        .unwrap_or_else(|| "Invalid".to_string())
}

fn format_expiration(exp: Option<i64>) -> String {
    exp.map(format_timestamp).unwrap_or_else(|| "Never".to_string())
}

fn format_status(invite: &InviteCode) -> String {
    if invite.is_used() {
        format!("Used by {}", invite.used_by.unwrap())
    } else if invite.is_expired(current_timestamp()) {
        "Expired".to_string()
    } else {
        "Active".to_string()
    }
}

fn current_timestamp() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64
}

fn print_usage() -> Result<()> {
    println!("Dialect Coach Admin CLI\n");
    println!("Commands:");
    println!("  generate-invite [--expires-days N]  Generate new invite code");
    println!("  list-invites                        List all invite codes");
    println!("  delete-invite <code>                Delete an invite code");
    Ok(())
}

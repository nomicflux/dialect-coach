use anyhow::Result;
use dialect_coach_shared::*;
use reqwest::StatusCode;
use reqwest::blocking::Client;
use std::env;

fn main() -> Result<()> {
    let args: Vec<String> = env::args().collect();
    let (client, backend_url, admin_token) = get_admin_config()?;

    match args.get(1).map(String::as_str) {
        Some("generate-invite") => generate_invite(&client, &backend_url, &admin_token, &args),
        Some("list-invites") => list_invites(&client, &backend_url, &admin_token),
        Some("delete-invite") => delete_invite(&client, &backend_url, &admin_token, &args),
        _ => print_usage(),
    }
}

fn get_admin_config() -> Result<(Client, String, String)> {
    let backend_url =
        env::var("BACKEND_URL").unwrap_or_else(|_| "http://localhost:3000".to_string());
    let admin_token =
        env::var("ADMIN_TOKEN").expect("ADMIN_TOKEN environment variable must be set");
    let client = Client::new();
    Ok((client, backend_url, admin_token))
}

fn generate_invite(
    client: &Client,
    backend_url: &str,
    admin_token: &str,
    args: &[String],
) -> Result<()> {
    let expires_days = parse_expires_days(args)?;
    let request = CreateInviteRequest { expires_days };

    let response = client
        .post(format!("{}/admin/api/invites", backend_url))
        .header("Authorization", format!("Bearer {}", admin_token))
        .json(&request)
        .send();

    match response {
        Ok(resp) => {
            if resp.status() == StatusCode::CREATED {
                let invite: InviteResponse = resp.json()?;
                format_invite_response(&invite)?;
            } else {
                handle_status_error(resp.status(), resp.text().unwrap_or_default())?;
            }
        }
        Err(e) => handle_http_error(backend_url, e)?,
    }
    Ok(())
}

fn list_invites(client: &Client, backend_url: &str, admin_token: &str) -> Result<()> {
    let response = client
        .get(format!("{}/admin/api/invites", backend_url))
        .header("Authorization", format!("Bearer {}", admin_token))
        .send();

    match response {
        Ok(resp) => {
            if resp.status() == StatusCode::OK {
                let list: InviteListResponse = resp.json()?;
                println!("Total invite codes: {}\n", list.invites.len());
                for item in list.invites {
                    format_invite_list_item(&item)?;
                }
            } else {
                handle_status_error(resp.status(), resp.text().unwrap_or_default())?;
            }
        }
        Err(e) => handle_http_error(backend_url, e)?,
    }
    Ok(())
}

fn delete_invite(
    client: &Client,
    backend_url: &str,
    admin_token: &str,
    args: &[String],
) -> Result<()> {
    let code = extract_code(args)?;
    let response = client
        .delete(format!("{}/admin/api/invites/{}", backend_url, code))
        .header("Authorization", format!("Bearer {}", admin_token))
        .send();

    match response {
        Ok(resp) => {
            if resp.status() == StatusCode::NO_CONTENT {
                println!("Deleted invite code: {}", code);
            } else if resp.status() == StatusCode::NOT_FOUND {
                println!("Error: Invite code not found");
                std::process::exit(1);
            } else {
                handle_status_error(resp.status(), resp.text().unwrap_or_default())?;
            }
        }
        Err(e) => handle_http_error(backend_url, e)?,
    }
    Ok(())
}

fn extract_code(args: &[String]) -> Result<String> {
    args.get(2)
        .cloned()
        .ok_or_else(|| anyhow::anyhow!("delete-invite requires a code"))
}

fn parse_expires_days(args: &[String]) -> Result<Option<u32>> {
    match args.iter().position(|a| a == "--expires-days") {
        Some(idx) => {
            let days: u32 = args
                .get(idx + 1)
                .ok_or_else(|| anyhow::anyhow!("--expires-days requires a number"))?
                .parse()?;
            Ok(Some(days))
        }
        None => Ok(None),
    }
}

fn format_invite_response(invite: &InviteResponse) -> Result<()> {
    println!("Code: {}", invite.code);
    println!("Created: {}", format_timestamp(invite.created_at));
    println!("Expires: {}", format_expiration(invite.expires_at));
    println!();
    Ok(())
}

fn format_invite_list_item(item: &InviteListItem) -> Result<()> {
    println!("Code: {}", item.code);
    println!("Created: {}", format_timestamp(item.created_at));
    println!("Expires: {}", format_expiration(item.expires_at));
    println!("Status: {}", item.status);
    println!();
    Ok(())
}

fn format_timestamp(ts: i64) -> String {
    use chrono::{DateTime, Utc};
    DateTime::from_timestamp(ts, 0)
        .map(|dt: DateTime<Utc>| dt.format("%Y-%m-%d %H:%M:%S UTC").to_string())
        .unwrap_or_else(|| "Invalid".to_string())
}

fn format_expiration(exp: Option<i64>) -> String {
    exp.map(format_timestamp)
        .unwrap_or_else(|| "Never".to_string())
}

fn handle_http_error(backend_url: &str, _error: reqwest::Error) -> Result<()> {
    eprintln!(
        "Error: Failed to connect to backend at {}. Is the server running?",
        backend_url
    );
    std::process::exit(1);
}

fn handle_status_error(status: StatusCode, body: String) -> Result<()> {
    match status {
        StatusCode::UNAUTHORIZED => {
            eprintln!("Error: Authentication failed. Please check your ADMIN_TOKEN.");
        }
        StatusCode::NOT_FOUND => {
            eprintln!("Error: Invite code not found.");
        }
        StatusCode::INTERNAL_SERVER_ERROR => {
            eprintln!("Error: Server error: {}", body);
        }
        _ => {
            eprintln!("Error: HTTP {} - {}", status, body);
        }
    }
    std::process::exit(1);
}

fn print_usage() -> Result<()> {
    println!("Dialect Coach Admin CLI\n");
    println!("Commands:");
    println!("  generate-invite [--expires-days N]  Generate new invite code");
    println!("  list-invites                        List all invite codes");
    println!("  delete-invite <code>                Delete an invite code");
    println!();
    println!("Environment variables:");
    println!("  ADMIN_TOKEN (required)              Authentication token");
    println!("  BACKEND_URL (default: http://localhost:3000)");
    Ok(())
}

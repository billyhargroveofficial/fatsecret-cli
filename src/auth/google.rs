//! One-time Google sign-in through FatSecret's own website and mobile endpoint.
//! Google credentials never go through argv, environment variables or stdout.

use std::io::IsTerminal;
use std::path::Path;
use std::time::Duration;

use reqwest::Client;
use serde_json::{Value, json};
use tokio::io::AsyncReadExt;

use super::{LoginOk, Triple};
use crate::config::AppConfig;
use crate::error::{AppError, Result};

const MAX_TOKEN_BYTES: usize = 16_384;

fn validate_token(token: &str) -> Result<&str> {
    let token = token.trim();
    let parts: Vec<_> = token.split('.').collect();
    if token.len() > MAX_TOKEN_BYTES
        || parts.len() != 3
        || parts.iter().any(|p| {
            p.is_empty()
                || !p
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
        })
    {
        return Err(AppError::Login("expected a Google ID token (JWT)".into()));
    }
    // This checks transport shape only. FatSecret verifies Google's signature,
    // expiry and audience; unverified claims are never used as identity here.
    Ok(token)
}

pub async fn read_stdin_token() -> Result<String> {
    if std::io::stdin().is_terminal() {
        return Err(AppError::Login(
            "--token-stdin requires piped input; use `auth google` for browser sign-in".into(),
        ));
    }
    let mut text = String::new();
    tokio::io::stdin()
        .take((MAX_TOKEN_BYTES + 1) as u64)
        .read_to_string(&mut text)
        .await?;
    Ok(validate_token(&text)?.to_owned())
}

/// Observed in Android 11.8.0.6: LinkUserWithGoogleDTO and path_link_with_google.
/// `checkUserExists: false` selects sign-in. The `true` registration check rejects
/// an existing account with "This social account already exists" (HTTP 400).
pub async fn login(http: &Client, app: &AppConfig, token: &str, device_id: &str) -> Result<Triple> {
    let token = validate_token(token)?;
    let endpoint = url::Url::parse(&app.google_auth_url)
        .map_err(|_| AppError::Login("invalid Google authentication URL".into()))?;
    let loopback = matches!(
        endpoint.host_str(),
        Some("127.0.0.1" | "localhost" | "[::1]")
    );
    if endpoint.scheme() != "https" && !(endpoint.scheme() == "http" && loopback) {
        return Err(AppError::Login(
            "Google authentication requires HTTPS".into(),
        ));
    }
    let response = http
        .post(endpoint)
        .header("fs_device", "android")
        .header("fs_device_type", "android")
        .header("fs_app_version", &app.app_version)
        .header("app_version", &app.app_version)
        .header("device", "6")
        .header("unit", "kj")
        .header("c_d", device_id)
        .header("c_desc", device_id)
        .json(&json!({
            "token": token,
            "checkUserExists": false,
            "deviceIdentifier": app.device_model,
        }))
        .send()
        .await?;
    let status = response.status();
    let value: Value = response.json().await.map_err(|_| {
        AppError::Login(format!("unexpected Google login response (HTTP {status})"))
    })?;
    if !status.is_success() || value.get("error").is_some() {
        // Do not echo arbitrary remote error text: it could contain the token.
        let kind = value
            .pointer("/error/typeId")
            .and_then(Value::as_i64)
            .map(|n| format!(", type {n}"))
            .unwrap_or_default();
        return Err(AppError::Login(format!(
            "Google sign-in rejected (HTTP {status}{kind}); use the Google account already linked to FatSecret and sign in again"
        )));
    }
    if value.get("isLinked").and_then(Value::as_bool) != Some(true) {
        return Err(AppError::Login(
            "Google account is not linked to an existing FatSecret account; no credentials saved"
                .into(),
        ));
    }
    let ok: LoginOk = serde_json::from_value(value).map_err(|_| {
        AppError::Login("incomplete Google login response; no credentials saved".into())
    })?;
    if ok.server_id <= 0 || ok.secret_key.trim().is_empty() || ok.device_key.trim().is_empty() {
        return Err(AppError::Login(
            "empty Google login credentials; no credentials saved".into(),
        ));
    }
    Ok(Triple {
        server_id: ok.server_id,
        secret_key: ok.secret_key,
        device_key: ok.device_key,
        username: ok.username.unwrap_or_default(),
    })
}

/// Playwriter controls an existing Chrome only. This function starts no browser
/// or resident service; every owned tab and the session are cleaned up on exit.
pub async fn browser_token(timeout_secs: u64) -> Result<String> {
    let cancel = cancellation()?;
    tokio::pin!(cancel);
    let scratch = tempfile::Builder::new()
        .prefix("fatsecret-google-")
        .tempdir()?;
    let dir = scratch.path();
    let token_path = dir.join("google-token");
    let start = format!(
        "const tokenPath = {};\n{}",
        serde_json::to_string(&token_path)?,
        include_str!("google_start.js")
    );
    std::fs::write(dir.join("start.js"), start)?;
    std::fs::write(dir.join("close.js"), include_str!("google_close.js"))?;
    let output = playwriter(dir, &["session", "new"]).await?;
    let session = session_id(&output)
        .ok_or_else(|| AppError::Login("could not read the new Playwriter session ID".into()))?;
    eprintln!(
        "Opening FatSecret Google sign-in in your existing Chrome (Playwriter session {session}). Choose the same Google account as in the mobile app."
    );
    let result = tokio::select! {
        _ = &mut cancel => Err(AppError::Login("Google sign-in cancelled".into())),
        result = async {
            playwriter(dir, &["-s", &session, "--timeout", "30000", "-f", "start.js"]).await?;
            eprintln!("Complete sign-in in Chrome. Waiting up to {timeout_secs} seconds; Ctrl-C cancels and closes this sign-in tab.");
            tokio::time::timeout(Duration::from_secs(timeout_secs), async {
                loop {
                    if token_path.exists() {
                        let token = std::fs::read_to_string(&token_path)?;
                        return Ok(validate_token(&token)?.to_owned());
                    }
                    if dir.join("closed").exists() {
                        return Err(AppError::Login("FatSecret sign-in tab was closed".into()));
                    }
                    tokio::time::sleep(Duration::from_millis(500)).await;
                }
            }).await.unwrap_or_else(|_| Err(AppError::Login("Google sign-in timed out".into())))
        } => result,
    };
    // Run cleanup even when navigation, authentication, Ctrl-C or SIGTERM failed.
    let closed = playwriter(
        dir,
        &["-s", &session, "--timeout", "30000", "-f", "close.js"],
    )
    .await;
    let deleted = playwriter(dir, &["session", "delete", &session]).await;
    if closed.is_err() || deleted.is_err() {
        eprintln!(
            "Could not finish browser cleanup for Playwriter session {session}; close its FatSecret sign-in tab if still open."
        );
    }
    result
}

fn session_id(output: &str) -> Option<String> {
    output.lines().find_map(|line| {
        let mut words = line.split_whitespace();
        if words.next()? != "Session" {
            return None;
        }
        let id = words.next()?;
        if !id.is_empty() && id.bytes().all(|b| b.is_ascii_digit()) {
            Some(id.to_owned())
        } else {
            None
        }
    })
}

async fn playwriter(dir: &Path, args: &[&str]) -> Result<String> {
    let executable =
        std::env::var_os("FATSECRET_PLAYWRITER").unwrap_or_else(|| "playwriter".into());
    let mut command = tokio::process::Command::new(executable);
    command
        .args(args)
        .current_dir(dir)
        .kill_on_drop(true)
        .env("NO_COLOR", "1")
        .stdin(std::process::Stdio::null());
    let output = tokio::time::timeout(Duration::from_secs(45), command.output()).await
        .map_err(|_| AppError::Login("Playwriter timed out; check the existing Chrome connection".into()))?
        .map_err(|_| AppError::Login("cannot run Playwriter; install it and connect it to an existing Chrome, or use --token-stdin".into()))?;
    if !output.status.success() {
        // Playwriter may print page URLs or page errors; keep them out of CLI logs.
        return Err(AppError::Login("Playwriter could not complete browser sign-in; check its connection to your existing Chrome and the FatSecret login page".into()));
    }
    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

fn cancellation() -> Result<impl std::future::Future<Output = ()>> {
    #[cfg(unix)]
    {
        use tokio::signal::unix::{SignalKind, signal};
        let mut interrupt = signal(SignalKind::interrupt())?;
        let mut terminate = signal(SignalKind::terminate())?;
        Ok(async move {
            tokio::select! { _ = interrupt.recv() => {}, _ = terminate.recv() => {} }
        })
    }
    #[cfg(not(unix))]
    Ok(async {
        let _ = tokio::signal::ctrl_c().await;
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn token_errors_never_echo_input() {
        for token in ["", "SECRET", "a.b.c.d", "a.\nsecret.c", ".b.c"] {
            assert_eq!(
                validate_token(token).unwrap_err().to_string(),
                "login failed: expected a Google ID token (JWT)"
            );
        }
        assert_eq!(validate_token(" a.b.c\n").unwrap(), "a.b.c");
        assert!(validate_token(&format!("{}.b.c", "a".repeat(MAX_TOKEN_BYTES))).is_err());
    }

    #[test]
    fn parses_only_announced_session() {
        assert_eq!(
            session_id("Discovering Chrome\nSession 195 created (direct CDP). Use with: ...\n"),
            Some("195".into())
        );
        assert_eq!(session_id("Error: session 195 unavailable"), None);
    }
}

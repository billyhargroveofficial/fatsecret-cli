//! Mobile session authentication (password or Google sign-in).
//!
//! Login exchanges `userName` + `password` for a server-issued triple
//! (`serverId`, `secretKey`, `deviceKey`) which is then sent as request
//! headers. Storage is a portable 0600 file, never the OS keychain
//! (see `spec://common/prop-000#secrets`).

pub mod device;
pub mod google;

use std::path::PathBuf;

use directories::BaseDirs;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::error::{AppError, Result};

/// Server-issued login triple. `secret_key` and `device_key` go out as the
/// `c_s` / `c_d` headers; `server_id` as `c_id`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Triple {
    pub server_id: i64,
    pub secret_key: String,
    pub device_key: String,
    #[serde(default)]
    pub username: String,
}

#[derive(Debug, Deserialize)]
struct LoginOk {
    #[serde(rename = "serverId")]
    server_id: i64,
    #[serde(rename = "deviceKey")]
    device_key: String,
    #[serde(rename = "secretKey")]
    secret_key: String,
    #[serde(rename = "userName")]
    username: Option<String>,
}

#[derive(Debug, Deserialize)]
struct LoginFailed {
    error: LoginErrorBody,
}

#[derive(Debug, Deserialize)]
struct LoginErrorBody {
    #[serde(rename = "typeId")]
    code: i64,
    message: String,
}

/// `POST {auth_url}` with `{userName, password, deviceIdentifier}`.
/// Returns the triple or [`AppError::Login`] with the server message.
pub async fn login(
    http: &Client,
    auth_url: &str,
    username: &str,
    password: &str,
    device_model: &str,
    app_version: &str,
    device_id: Option<&str>,
) -> Result<Triple> {
    let body = json!({
        "userName": username,
        "password": password,
        "deviceIdentifier": device_model,
    });
    let mut req = http
        .post(auth_url)
        .header("Content-Type", "application/json")
        .header("fs_device", "android")
        .header("fs_device_type", "android")
        .header("fs_app_version", app_version)
        .header("app_version", app_version)
        .header("device", "6")
        .header("unit", "kj");
    if let Some(id) = device_id {
        req = req.header("c_d", id).header("c_desc", id);
    }
    let status = req.json(&body).send().await?;
    let code = status.status();
    let value: Value = status.json().await?;
    if !code.is_success() {
        let message = serde_json::from_value::<LoginFailed>(value)
            .map(|e| format!("{} (type {})", e.error.message, e.error.code))
            .unwrap_or_else(|_| format!("login rejected (HTTP {code})"));
        return Err(AppError::Login(message));
    }
    let ok: LoginOk = serde_json::from_value(value)
        .map_err(|e| AppError::Login(format!("unexpected login response: {e}")))?;
    Ok(Triple {
        server_id: ok.server_id,
        secret_key: ok.secret_key,
        device_key: ok.device_key,
        username: ok.username.unwrap_or_else(|| username.to_string()),
    })
}

/// Read a password from the TTY without echo. Never from flags or env.
pub fn prompt_password() -> Result<String> {
    rpassword::prompt_password("Password: ").map_err(AppError::Io)
}

/// Portable triple store: JSON file, 0600 on Unix. Env triple
/// (`FATSECRET_SERVER_ID` + `FATSECRET_SECRET_KEY` + `FATSECRET_DEVICE_KEY`)
/// overrides the file for automation; `FATSECRET_CREDENTIALS` overrides the
/// file path itself (tests, custom layouts).
pub struct FileStore {
    path: PathBuf,
}

impl FileStore {
    pub fn platform() -> Result<Self> {
        if let Ok(path) = std::env::var("FATSECRET_CREDENTIALS") {
            return Ok(Self {
                path: PathBuf::from(path),
            });
        }
        default_credentials_path()
            .map(|path| Self { path })
            .ok_or_else(|| AppError::Msg("no config dir available".to_string()))
    }

    pub fn load(&self) -> Result<Triple> {
        if let (Ok(id), Ok(secret), Ok(device)) = (
            std::env::var("FATSECRET_SERVER_ID"),
            std::env::var("FATSECRET_SECRET_KEY"),
            std::env::var("FATSECRET_DEVICE_KEY"),
        ) {
            let server_id = id
                .parse::<i64>()
                .map_err(|_| AppError::Msg("FATSECRET_SERVER_ID is not a number".to_string()))?;
            let username = std::env::var("FATSECRET_USERNAME").unwrap_or_default();
            return Ok(Triple {
                server_id,
                secret_key: secret,
                device_key: device,
                username,
            });
        }
        let text = std::fs::read_to_string(&self.path).map_err(|_| AppError::NotLoggedIn)?;
        serde_json::from_str(&text).map_err(|_| AppError::NotLoggedIn)
    }

    pub fn save(&self, triple: &Triple) -> Result<()> {
        use std::io::Write;
        if let Some(parent) = self.path.parent().filter(|p| !p.as_os_str().is_empty()) {
            std::fs::create_dir_all(parent)?;
        }
        // Restrictive permissions from creation, then atomic replacement: no
        // world-readable window and no truncated credentials on a failed write.
        let parent = self
            .path
            .parent()
            .filter(|p| !p.as_os_str().is_empty())
            .unwrap_or_else(|| std::path::Path::new("."));
        let mut file = tempfile::NamedTempFile::new_in(parent)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            file.as_file()
                .set_permissions(std::fs::Permissions::from_mode(0o600))?;
        }
        file.write_all(serde_json::to_string_pretty(triple)?.as_bytes())?;
        file.as_file().sync_all()?;
        file.persist(&self.path)
            .map_err(|e| AppError::Io(e.error))?;
        Ok(())
    }

    pub fn delete(&self) -> Result<()> {
        match std::fs::remove_file(&self.path) {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(AppError::Io(e)),
        }
    }
}

fn default_credentials_path() -> Option<PathBuf> {
    BaseDirs::new().map(|b| {
        b.config_dir()
            .join("fatsecret-cli")
            .join("credentials.json")
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_login_success_shape() {
        let v: Value = serde_json::from_str(
            r#"{"isLinked":true,"serverId":42,"deviceKey":"d","secretKey":"s","userName":"u","email":"e"}"#,
        )
        .unwrap();
        let ok: LoginOk = serde_json::from_value(v).unwrap();
        assert_eq!(ok.server_id, 42);
        assert_eq!(ok.device_key, "d");
    }

    #[test]
    fn parses_login_error_shape() {
        let v: Value = serde_json::from_str(
            r#"{"error":{"typeId":1,"message":"Your email address/password combination is incorrect. Please try again, or use Forgot Password."}}"#,
        )
        .unwrap();
        let err: LoginFailed = serde_json::from_value(v).unwrap();
        assert_eq!(err.error.code, 1);
        assert!(err.error.message.contains("incorrect"));
    }
}

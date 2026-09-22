use crate::api::register_body;
use crate::auth::device::{ensure_device_id, today_ymd};
use crate::auth::{FileStore, login, prompt_password};
use crate::cli::{AuthAction, AuthArgs, OutputFormat};
use crate::commands::{app_client, http_client};
use crate::config::AppConfig;
use crate::error::{AppError, Result};
use crate::output::emit;

pub async fn run(app: &AppConfig, format: OutputFormat, args: AuthArgs) -> Result<()> {
    let store = FileStore::platform()?;
    match args.action {
        AuthAction::Google {
            token_stdin,
            timeout,
        } => {
            // Authentication never follows redirects with a Google credential.
            let http = reqwest::Client::builder()
                .redirect(reqwest::redirect::Policy::none())
                .connect_timeout(std::time::Duration::from_secs(15))
                .timeout(std::time::Duration::from_secs(45))
                .build()?;
            let token = if token_stdin {
                crate::auth::google::read_stdin_token().await?
            } else {
                crate::auth::google::browser_token(timeout).await?
            };
            let device_id = ensure_device_id(&http, app.device_id.as_deref()).await?;
            let triple = crate::auth::google::login(&http, app, &token, &device_id).await?;
            store.save(&triple)?;
            let human = format!("logged in as {} (Google)", triple.username);
            emit(
                format,
                &human,
                &human,
                &serde_json::json!({"status":"logged in","username":triple.username,"method":"google"}),
            )
        }
        AuthAction::Login { username } => {
            let http = http_client();
            // Per-install identity first: explicit config, stored fid,
            // otherwise mint + persist (network, once per install).
            let device_id = ensure_device_id(&http, app.device_id.as_deref()).await?;
            let password = prompt_password()?;
            let triple = login(
                &http,
                &app.auth_url,
                username.trim(),
                &password,
                &app.device_model,
                &app.app_version,
                Some(&device_id),
            )
            .await?;
            store.save(&triple)?;
            let human = format!("logged in as {}", triple.username);
            emit(
                format,
                &human,
                &human,
                &serde_json::json!({"status":"logged in","username":triple.username}),
            )
        }
        AuthAction::Register {
            email,
            username,
            birth_date,
            gender,
            country,
            current_weight_kg,
            goal_weight_kg,
            height_cm,
            first_name,
        } => {
            let http = http_client();
            let device_id = ensure_device_id(&http, app.device_id.as_deref()).await?;
            let password = prompt_password()?;
            let body = register_body(
                email.trim(),
                username.trim(),
                &password,
                &gender,
                &birth_date,
                0,
                current_weight_kg,
                goal_weight_kg,
                0,
                height_cm,
                &today_ymd(),
                &country,
                None,
                None,
                None,
                &app.device_model,
                first_name.as_deref(),
            );
            let client = app_client(app)?;
            client.register(&body, Some(&device_id)).await?;
            let human = format!("registered; check {email} for confirmation");
            emit(
                format,
                &human,
                &human,
                &serde_json::json!({"status":"registered","email":email}),
            )
        }
        AuthAction::ForgotPassword { email } => {
            let http = http_client();
            let device_id = ensure_device_id(&http, app.device_id.as_deref()).await?;
            let client = app_client(app)?;
            client
                .forgot_password(email.trim(), Some(&device_id))
                .await?;
            let human = format!("reset email sent to {email}");
            emit(
                format,
                &human,
                &human,
                &serde_json::json!({"status":"reset email sent","email":email}),
            )
        }
        AuthAction::ResetPassword => {
            let code = rpassword::prompt_password("Reset code: ").map_err(AppError::Io)?;
            let password = prompt_password()?;
            let http = http_client();
            let device_id = ensure_device_id(&http, app.device_id.as_deref()).await?;
            let client = app_client(app)?;
            client
                .reset_password(code.trim(), &password, Some(&device_id))
                .await?;
            let human = "password reset; log in with the new password";
            emit(
                format,
                human,
                human,
                &serde_json::json!({"status":"password reset"}),
            )
        }
        AuthAction::Status => match store.load() {
            Ok(t) if t.username.is_empty() => emit(
                format,
                "logged in",
                "logged in",
                &serde_json::json!({"status":"logged in"}),
            ),
            Ok(t) => {
                let human = format!("logged in as {}", t.username);
                emit(
                    format,
                    &human,
                    &human,
                    &serde_json::json!({"status":"logged in","username":t.username}),
                )
            }
            Err(e) => {
                emit(
                    format,
                    "logged out",
                    "logged out",
                    &serde_json::json!({"status":"logged out"}),
                )?;
                Err(e)
            }
        },
        AuthAction::Logout => {
            store.delete()?;
            emit(
                format,
                "logged out",
                "logged out",
                &serde_json::json!({"status":"logged out"}),
            )
        }
    }
}

use thiserror::Error;

#[derive(Error, Debug)]
pub enum AppError {
    #[error("FatSecret error {code}: {message}")]
    Api { code: String, message: String },

    #[error("login failed: {0}")]
    Login(String),

    #[error(
        "not logged in: run `fatsecret-cli auth google` or `fatsecret-cli auth login <USERNAME>` first"
    )]
    NotLoggedIn,

    #[error("no device model: set FATSECRET_DEVICE_MODEL or device_model in the profile file")]
    NoDevice,

    #[error("{0}")]
    Msg(String),

    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error(transparent)]
    Http(#[from] reqwest::Error),

    #[error(transparent)]
    Json(#[from] serde_json::Error),
}

pub type Result<T> = std::result::Result<T, AppError>;

//! Real CLI against a local HTTP server: no Google account or internet needed.
use std::io::{Read, Write};
use std::net::TcpListener;
use std::sync::mpsc;
use std::time::{Duration, Instant};

use assert_cmd::Command;
use predicates::prelude::*;
use serde_json::{Value, json};

const TOKEN: &str = "fixture.payload.signature";

fn server(status: &str, body: &str) -> (String, mpsc::Receiver<String>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let url = format!("http://{}/google", listener.local_addr().unwrap());
    let response = format!(
        "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        let deadline = Instant::now() + Duration::from_secs(10);
        let mut stream = loop {
            match listener.accept() {
                Ok((stream, _)) => break stream,
                Err(_) if Instant::now() < deadline => {
                    std::thread::sleep(Duration::from_millis(10))
                }
                Err(_) => return,
            }
        };
        stream
            .set_read_timeout(Some(Duration::from_secs(5)))
            .unwrap();
        let mut data = Vec::new();
        let mut buf = [0; 4096];
        loop {
            let read = stream.read(&mut buf).unwrap();
            if read == 0 {
                break;
            }
            data.extend_from_slice(&buf[..read]);
            if let Some(pos) = data.windows(4).position(|w| w == b"\r\n\r\n") {
                let headers = String::from_utf8_lossy(&data[..pos]);
                let length: usize = headers
                    .lines()
                    .find_map(|line| {
                        line.to_ascii_lowercase()
                            .strip_prefix("content-length:")
                            .and_then(|s| s.trim().parse().ok())
                    })
                    .unwrap();
                if data.len() >= pos + 4 + length {
                    break;
                }
            }
        }
        stream.write_all(response.as_bytes()).unwrap();
        let _ = tx.send(String::from_utf8(data).unwrap());
    });
    (url, rx)
}

fn cli(dir: &std::path::Path, url: &str) -> Command {
    std::fs::write(dir.join("config.toml"), "").unwrap();
    let mut cmd = Command::cargo_bin("fatsecret-cli").unwrap();
    cmd.args([
        "--config",
        dir.join("config.toml").to_str().unwrap(),
        "--format",
        "json",
        "auth",
        "google",
    ])
    .env("FATSECRET_CREDENTIALS", dir.join("credentials.json"))
    .env("FATSECRET_DEVICE_ID", "fixture-installation")
    .env("FATSECRET_GOOGLE_AUTH_URL", url)
    .env("NO_PROXY", "127.0.0.1,localhost")
    .env("no_proxy", "127.0.0.1,localhost")
    .env_remove("FATSECRET_SERVER_ID")
    .env_remove("FATSECRET_SECRET_KEY")
    .env_remove("FATSECRET_DEVICE_KEY")
    .timeout(Duration::from_secs(15));
    cmd
}

fn success() -> Value {
    json!({"isLinked":true,"serverId":42,"deviceKey":"private-device","secretKey":"private-secret","userName":"existing-member"})
}

#[test]
fn exchanges_google_token_for_existing_account_and_stores_privately() {
    let dir = tempfile::tempdir().unwrap();
    let (url, request) = server("200 OK", &success().to_string());
    cli(dir.path(), &url)
        .arg("--token-stdin")
        .write_stdin(format!("{TOKEN}\n"))
        .assert()
        .success()
        .stdout(predicate::str::contains("existing-member"))
        .stdout(predicate::str::contains(TOKEN).not())
        .stdout(predicate::str::contains("private-secret").not())
        .stderr(predicate::str::contains(TOKEN).not());
    let request = request.recv_timeout(Duration::from_secs(2)).unwrap();
    let (headers, body) = request.split_once("\r\n\r\n").unwrap();
    let body: Value = serde_json::from_str(body).unwrap();
    assert_eq!(body["token"], TOKEN);
    assert_eq!(body["checkUserExists"], false);
    assert!(body.get("password").is_none());
    assert!(
        headers
            .to_ascii_lowercase()
            .contains("c_d: fixture-installation")
    );
    let path = dir.path().join("credentials.json");
    let saved: Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    assert_eq!(saved["server_id"], 42);
    assert_eq!(saved["secret_key"], "private-secret");
    assert!(!saved.to_string().contains(TOKEN));
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        assert_eq!(
            std::fs::metadata(path).unwrap().permissions().mode() & 0o777,
            0o600
        );
    }
}

#[test]
fn failed_login_preserves_existing_credentials_and_never_echoes_remote_secrets() {
    let cases = [
        (
            "401 Unauthorized",
            json!({"error":{"typeId":3,"message":TOKEN}}).to_string(),
        ),
        (
            "200 OK",
            json!({"error":{"typeId":3,"message":TOKEN}}).to_string(),
        ),
        ("200 OK", json!({"isLinked":false}).to_string()),
        ("200 OK", json!({"isLinked":true,"serverId":42}).to_string()),
        (
            "200 OK",
            json!({"isLinked":true,"serverId":0,"secretKey":"","deviceKey":""}).to_string(),
        ),
        ("502 Bad Gateway", format!("<html>{TOKEN}</html>")),
    ];
    for (status, body) in cases {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("credentials.json");
        std::fs::write(&path, "original-credentials").unwrap();
        let (url, request) = server(status, &body);
        cli(dir.path(), &url)
            .arg("--token-stdin")
            .write_stdin(TOKEN)
            .assert()
            .failure()
            .stdout(predicate::str::contains(TOKEN).not())
            .stderr(predicate::str::contains(TOKEN).not());
        request.recv_timeout(Duration::from_secs(2)).unwrap();
        assert_eq!(
            std::fs::read_to_string(path).unwrap(),
            "original-credentials"
        );
    }
}

#[test]
fn rejects_invalid_token_and_insecure_endpoint_before_network() {
    let dir = tempfile::tempdir().unwrap();
    cli(dir.path(), "http://example.invalid/google")
        .arg("--token-stdin")
        .write_stdin(TOKEN)
        .assert()
        .failure()
        .stderr(predicate::str::contains("requires HTTPS"));
    cli(dir.path(), "http://127.0.0.1:1/google")
        .arg("--token-stdin")
        .write_stdin("private-invalid-token")
        .assert()
        .failure()
        .stderr(predicate::str::contains("expected a Google ID token"))
        .stderr(predicate::str::contains("private-invalid-token").not());
    assert!(!dir.path().join("credentials.json").exists());
}

#[test]
fn does_not_follow_authentication_redirects() {
    let dir = tempfile::tempdir().unwrap();
    let (url, request) = server("302 Found\r\nLocation: http://127.0.0.1:1/collect", "{}");
    cli(dir.path(), &url)
        .arg("--token-stdin")
        .write_stdin(TOKEN)
        .assert()
        .failure()
        .stderr(predicate::str::contains("HTTP 302"));
    request.recv_timeout(Duration::from_secs(2)).unwrap();
    assert!(!dir.path().join("credentials.json").exists());
}

#[cfg(unix)]
fn fake_playwriter(dir: &std::path::Path) -> std::path::PathBuf {
    use std::os::unix::fs::PermissionsExt;
    let path = dir.join("playwriter");
    std::fs::write(
        &path,
        r#"#!/bin/sh
case "$*" in
  'session new') printf 'Session 123 created (direct CDP).\n' ;;
  *start.js)
    printf 'start\n' >> "$TEST_PW_LOG"
    case "$TEST_PW_MODE" in
      fail) exit 1 ;;
      wait) : ;;
      *) printf 'fixture.payload.signature' > google-token ;;
    esac ;;
  *close.js) printf 'close\n' >> "$TEST_PW_LOG" ;;
  'session delete 123') printf 'delete\n' >> "$TEST_PW_LOG" ;;
  *) exit 2 ;;
esac
"#,
    )
    .unwrap();
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o700)).unwrap();
    path
}

#[test]
#[cfg(unix)]
fn browser_session_is_cleaned_on_success_failure_and_timeout() {
    for mode in ["success", "fail", "wait"] {
        let dir = tempfile::tempdir().unwrap();
        let (url, _request) = server("200 OK", &success().to_string());
        let log = dir.path().join("lifecycle.log");
        let mut cmd = cli(dir.path(), &url);
        cmd.args(["--timeout", "1"])
            .env("FATSECRET_PLAYWRITER", fake_playwriter(dir.path()))
            .env("TEST_PW_LOG", &log)
            .env("TEST_PW_MODE", mode);
        if mode == "success" {
            cmd.assert().success();
        } else {
            cmd.assert().failure();
        }
        assert_eq!(
            std::fs::read_to_string(log).unwrap(),
            "start\nclose\ndelete\n"
        );
    }
}

#[test]
#[cfg(unix)]
fn sigterm_cleans_browser_session_before_exiting() {
    use std::process::Stdio;
    let dir = tempfile::tempdir().unwrap();
    let log = dir.path().join("lifecycle.log");
    let mut command = cli(dir.path(), "http://127.0.0.1:1/google");
    command
        .env("FATSECRET_PLAYWRITER", fake_playwriter(dir.path()))
        .env("TEST_PW_LOG", &log)
        .env("TEST_PW_MODE", "wait");
    let mut process = std::process::Command::new(command.get_program());
    process.args(command.get_args());
    for (key, value) in command.get_envs() {
        match value {
            Some(value) => {
                process.env(key, value);
            }
            None => {
                process.env_remove(key);
            }
        }
    }
    let mut child = process
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    let ready = Instant::now() + Duration::from_secs(5);
    while !log.exists() && Instant::now() < ready {
        std::thread::sleep(Duration::from_millis(20));
    }
    if !log.exists() {
        let _ = child.kill();
        let _ = child.wait();
        panic!("fake browser did not start");
    }
    assert!(
        std::process::Command::new("kill")
            .args(["-TERM", &child.id().to_string()])
            .status()
            .unwrap()
            .success()
    );
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        if let Some(status) = child.try_wait().unwrap() {
            assert!(!status.success());
            break;
        }
        if Instant::now() >= deadline {
            child.kill().unwrap();
            child.wait().unwrap();
            panic!("CLI did not stop after SIGTERM");
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    assert_eq!(
        std::fs::read_to_string(log).unwrap(),
        "start\nclose\ndelete\n"
    );
    assert!(!dir.path().join("credentials.json").exists());
}

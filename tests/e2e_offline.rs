//! Offline end-to-end matrix: the real binary, hermetic environment.
//!
//! No credentials, no network. Every help screen must render; every
//! network-dependent path must fail closed (`not logged in`, nonzero exit)
//! instead of panicking or hanging.

use assert_cmd::Command;
use predicates::prelude::*;

/// Binary with all credential sources pointed at nothing.
fn offline() -> Command {
    let mut cmd = Command::cargo_bin("fatsecret-cli").unwrap();
    cmd.env("XDG_CONFIG_HOME", "/nonexistent-xdg-fatsecret-cli")
        .env(
            "FATSECRET_CREDENTIALS",
            "/nonexistent-creds-fatsecret-cli.json",
        )
        .env_remove("FATSECRET_SERVER_ID")
        .env_remove("FATSECRET_DEVICE_ID");
    cmd
}

fn help_ok(args: &[&str], marker: &str) {
    let mut cmd = offline();
    cmd.args(args)
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains(marker).or(predicate::str::contains("Usage")));
}

#[test]
fn all_top_level_help_renders() {
    for cmd in [
        "auth",
        "foods",
        "recipes",
        "diary",
        "weight",
        "rdi",
        "exercise",
        "water",
        "meals",
        "meal-plans",
        "account",
        "settings",
        "notifications",
        "feed",
        "learning",
        "food-groups",
        "config",
        "completions",
    ] {
        help_ok(&[cmd], "Usage");
    }
}

#[test]
fn all_leaf_help_renders() {
    // (args, marker unique to that screen)
    let screens: &[(&[&str], &str)] = &[
        (&["auth", "login"], "USERNAME"),
        (&["auth", "google"], "--token-stdin"),
        (&["auth", "register"], "password"),
        (&["auth", "forgot-password"], "user"),
        (&["auth", "reset-password"], "code"),
        (&["auth", "status"], "login state"),
        (&["auth", "logout"], "credentials"),
        (&["foods", "search"], "QUERY"),
        (&["foods", "search"], "--market"),
        (&["foods", "get"], "food id"),
        (&["foods", "popular"], "food ids"),
        (&["foods", "vote-preference"], "--vote"),
        (&["foods", "barcode"], "GTIN"),
        (&["foods", "preference-types"], "types"),
        (&["recipes", "search"], "QUERY"),
        (&["recipes", "get"], "recipe"),
        (&["recipes", "categories"], "Usage"),
        (&["recipes", "cookbook-search"], "QUERY"),
        (&["recipes", "cookbook-count"], "Usage"),
        (&["diary", "add"], "--food-id"),
        (&["diary", "day"], "history supported"),
        (&["diary", "cp"], "--from"),
        (&["weight", "log"], "--goal-kg"),
        (&["rdi", "show"], "Usage"),
        (&["rdi", "save"], "--weight-kg"),
        (&["exercise", "day"], "Usage"),
        (&["exercise", "types"], "Usage"),
        (&["exercise", "log"], "--type-id"),
        (&["water", "day"], "Usage"),
        (&["water", "log"], "--goal-ml"),
        (&["meals", "ls"], "Usage"),
        (&["meals", "show"], "Usage"),
        (&["meals", "create"], "--title"),
        (&["meals", "save"], "--title"),
        (&["meals", "rm"], "Usage"),
        (&["meals", "log"], "meal"),
        (&["meals", "add-item"], "--meal-id"),
        (&["meals", "rm-item"], "--item-id"),
        (&["meals", "quickpicks"], "Usage"),
        (&["meals", "duplicate"], "--title"),
        (&["meal-plans", "save"], "--entry"),
        (&["meal-plans", "schedule"], "--insert"),
        (&["account", "show"], "Usage"),
        (&["account", "settings"], "Usage"),
        (&["account", "change-username"], "Usage"),
        (&["settings", "attributes"], "Usage"),
        (&["settings", "locale"], "Usage"),
        (&["notifications", "ls"], "Usage"),
        (&["feed", "blocks"], "Usage"),
        (&["feed", "blocking"], "Usage"),
        (&["feed", "block"], "user"),
        (&["learning", "progress"], "Usage"),
        (&["learning", "content"], "Usage"),
        (&["learning", "bookmark-course"], "course"),
        (&["learning", "complete-lesson"], "lesson"),
        (&["foods", "create"], "--metric-serving"),
        (&["recipes", "create"], "--prep-time"),
        (&["recipes", "save"], "--step"),
        (&["config", "show"], "Usage"),
        (&["config", "set"], "key"),
    ];
    for (args, marker) in screens {
        help_ok(args, marker);
    }
}

#[test]
fn network_paths_fail_closed_offline() {
    // Reads and writes alike: no credentials → `not logged in`, never a panic.
    let cases: &[&[&str]] = &[
        &["foods", "search", "oats"],
        &["foods", "get", "39715"],
        &["recipes", "search", "oats"],
        &["diary", "day"],
        &[
            "diary",
            "add",
            "--food-id",
            "39715",
            "--serving-id",
            "62446",
            "--meal",
            "breakfast",
            "--date",
            "2026-09-05",
        ],
        &["diary", "rm", "1", "--date", "2026-09-05"],
        &["weight", "log", "70"],
        &["exercise", "day"],
        &["exercise", "types"],
        &["exercise", "log", "--type-id", "77", "--mins", "30"],
        &["water", "day"],
        &["water", "log", "500"],
        &["meals", "ls"],
        &["meals", "quickpicks"],
        &["account", "show"],
        &["settings", "attributes"],
        &["notifications", "ls"],
        &["feed", "blocks"],
        &["learning", "progress"],
        &["food-groups"],
    ];
    for args in cases {
        let mut cmd = offline();
        cmd.args(*args).assert().failure().stderr(
            predicate::str::contains("not logged in")
                .or(predicate::str::contains("no device model")),
        );
    }
}

#[test]
fn date_matrix_rejects_offline() {
    for bad in ["not-a-date", "2026-13-45", "2023-02-29", "2026-04-31"] {
        offline()
            .args(["diary", "rm", "1", "--date", bad])
            .assert()
            .failure()
            .stderr(predicate::str::contains("bad date"))
            .stderr(predicate::str::contains("want YYYY-MM-DD"));
    }
    // A valid date passes validation and fails later at auth, never on the date.
    offline()
        .args(["diary", "rm", "1", "--date", "2024-02-29"])
        .assert()
        .failure()
        .stderr(
            predicate::str::contains("not logged in")
                .or(predicate::str::contains("no device model")),
        );
}

#[test]
fn auth_status_json_logged_out_offline() {
    offline()
        .args(["auth", "status", "--format", "json"])
        .assert()
        .failure()
        .stdout(predicate::str::contains("logged out"))
        .stderr(predicate::str::contains("not logged in"));
}

#[test]
fn bad_format_and_unknown_command_rejected() {
    offline()
        .args(["--format", "yaml", "diary", "day"])
        .assert()
        .failure();
    offline().args(["frobnicate"]).assert().failure();
}

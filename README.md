# fatsecret-cli

English | [Русский](README.ru.md)

This fork adds Google sign-in to [vasyza/fatsecret-cli](https://github.com/vasyza/fatsecret-cli). Run `fatsecret-cli auth google` once, then use the normal food search and diary commands. No emulator or background CLI process is needed.

A Rust command-line client for FatSecret. Search foods and recipes, look up barcodes, add food diary entries, record weight, and read account data from your terminal.

The CLI uses the mobile app's authentication and endpoints, not the public FatSecret Platform API. You sign in with a FatSecret account; no developer application or Platform API subscription is required. This is an unofficial client and is not affiliated with or endorsed by FatSecret. Mobile endpoints can change without notice.

The goal is to cover the mobile app's functionality. Version 0.1.0 does not yet offer full parity: some commands are read-only, some writes are rejected by the server, and several app features are absent. See [current coverage](#current-coverage) before relying on a workflow.

## Contents

- [Installation](#installation)
- [Quick start](#quick-start)
- [Current coverage](#current-coverage)
- [Command reference](#command-reference)
- [Output and scripting](#output-and-scripting)
- [Configuration](#configuration)
- [Credentials and privacy](#credentials-and-privacy)
- [Shell completions](#shell-completions)
- [Troubleshooting](#troubleshooting)
- [Development](#development)
- [License](#license)

## Installation

Build from a local checkout with a current stable Rust toolchain and Cargo. Install Rust through [rustup](https://rustup.rs/) if needed.

```sh
cargo install --path . --locked
fatsecret-cli --version
```

Cargo normally installs the executable in `~/.cargo/bin`; that directory must be on `PATH`. For a build that stays inside the checkout:

```sh
cargo build --release --locked
./target/release/fatsecret-cli --help
```

The manifest requires Rust 1.88 or newer. Use current stable with the locked dependencies.

Network commands require access to FatSecret. Initial device registration also contacts Firebase Installations. The client uses rustls for TLS. Credential-file permissions are restricted to the owner on Unix; other operating systems need appropriate filesystem access controls. This is not a claim that every platform has been tested.

## Quick start

For an account linked to Google, use an existing Chrome with [Playwriter](https://github.com/remorses/playwriter) connected:

```sh
fatsecret-cli auth google
fatsecret-cli config set market_locale RU
fatsecret-cli config set language_locale ru
fatsecret-cli foods search "гречка" --format json
```

The command opens FatSecret's own Google sign-in button. Choose the same Google account you use in the mobile app. It saves the resulting FatSecret session and closes its temporary tabs, including the Google popup. The existing browser stays running; later searches and diary operations do not use it. No password reset or developer API subscription is needed. See [Google authentication details](docs/google-auth.md).

Log in from an interactive terminal. The password prompt does not echo your input.

```sh
fatsecret-cli auth login you@example.com
fatsecret-cli auth status
fatsecret-cli foods search "oatmeal"
```

Inspect a result before adding it. Food IDs and serving IDs are different identifiers; a serving ID must belong to the selected food.

```sh
fatsecret-cli foods get 39715 --format json
fatsecret-cli foods popular 39715 --format json
```

Replace these sample IDs with values from your results. In a POSIX shell, use an explicit local date when adding an entry:

```sh
fatsecret-cli diary add \
  --food-id 39715 \
  --serving-id 62446 \
  --meal breakfast \
  --units 1 \
  --date "$(date +%F)" \
  --format json

fatsecret-cli diary day --format json
```

This changes your real account. Keep the add response if you may need to delete the entry: the day response does not reliably expose entry IDs. Confirm the date and entry in the mobile app. The explicit date avoids the current default-date calculation bug described under [troubleshooting](#troubleshooting).

Record weight in kilograms:

```sh
fatsecret-cli weight log 75.2 --goal-kg 70
```

This updates both current weight and the supplied goal. Omit `--goal-kg` to retain the account goal when the API returns it as a number. If that value is absent or not numeric, the CLI uses the new current weight as the goal.

## Current coverage

The status below distinguishes implemented commands from observed live behavior. Previous live checks do not guarantee that a private endpoint still works for every account or market.

| Area | Available behavior | Limits and observed status |
| --- | --- | --- |
| Authentication | Google sign-in, password login, local status/logout, registration, password recovery | Google token exchange checked live against an existing account. Registration and reset completion are implemented without the same verification claim. |
| Foods | Search, details, popular servings, dietary-preference types/votes, barcode lookup | Search, details, popular servings, types, and barcode lookup have live-check evidence. Vote requests are implemented; this does not establish every type/value combination. |
| Recipes | Search, details, categories, cookbook search/count | Search, details, categories, and count have been checked live. Cookbook search is implemented without the same verification claim. |
| Food diary | Day read with history, add, delete and copy | Add/delete/copy verified live. Day lists entry IDs, macros and totals. No month view. Date caveats apply. |
| Weight | Record current weight and goal | A write was accepted live. No history or date selector. |
| Exercise | Current day (with `--date` history), activity types, activity logging | Logging verified live; the request needs the app locale trio (`lang/mkt/device` + `userid`), now sent by the client. |
| Water | Current-day read and intake logging | Premium-gated: the account in live checks is `registered non-premium` and the server rejects both operations (299 `Could not save`). Verified via web sources that water tracking is a Premium feature. |
| Saved meals | List, details, quick picks, meal/item writes, log to diary | Reads have been checked live. Create/delete verified live over form POST (app transport); other writes share the transport but are unproven. |
| Account/settings | Details, account settings, rename, attributes, locale | Read commands have live-check evidence. Rename is implemented and changes the account immediately if accepted. General settings editing is absent. |
| Notifications | List | Read-only; checked live. |
| Social | Block-related user lists | Read-only; checked live with empty lists. CLI help labels `blocks` as users blocking you and `blocking` as users you block; direction has not been confirmed against populated lists. No feed posting or block/unblock commands. |
| Learning | Content, progress, bookmarks in the progress response | Reads have been checked live. No lesson-progress or bookmark writes. |
| Food groups | Static group data | Checked live; still requires login. |

Not implemented: favorites management, custom-food/image workflows, Apple/Facebook login, AI/image recognition, gamification/streak updates, and complete mobile-app parity. A command appearing in `--help` means it exists, not that its remote workflow is verified.

## Command reference

```text
fatsecret-cli [OPTIONS] <COMMAND>
fatsecret-cli <COMMAND> --help
fatsecret-cli <COMMAND> <SUBCOMMAND> --help
```

In the tables, uppercase names are values you supply and square brackets mark optional arguments. Do not type the brackets. Commands and flags are the same in both README languages.

All data commands require stored or environment-provided credentials, including food search and food groups. Help and completions do not require login. Authentication commands manage their own pre-login or local behavior.

### Global options

| Option | Meaning |
| --- | --- |
| `-c, --config PATH` | Select a TOML configuration file. Also settable through `FATSECRET_CONFIG`. |
| `--format human\|json\|plain` | Output format; default `human`. Authentication and completions are exceptions. |
| `--color auto\|always\|never` | Log color on stderr only; data output stays uncolored. Default `auto`. |
| `-v, --verbose` | When `RUST_LOG` is unset: `-v` is `info`, `-vv` is `debug`, no flag is `warn`. `RUST_LOG` wins when set. |
| `-h, --help` | Show help. |
| `-V, --version` | Show version. |

Global options can also follow subcommands. `RUST_LOG` overrides `-v` when set.

### Authentication

| Command | Behavior |
| --- | --- |
| `auth google [--timeout 600]` | Opens Google sign-in through Playwriter in an existing Chrome, stores the linked FatSecret session, and cleans up its tabs. |
| `auth google --token-stdin` | Advanced integration: reads a fresh Google ID token from a pipe; never supply tokens in command arguments. |
| `auth login USERNAME` | Accepts a username or email, prompts for a password, and stores login credentials. |
| `auth register EMAIL USERNAME --birth-date YYYY-MM-DD --gender GENDER [--country US]` | Creates an account; prompts for a password. Gender is passed through as supplied, for example `Male` or `Female`. |
| `auth forgot-password EMAIL` | Sends a password-reset email. |
| `auth reset-password` | Prompts for the reset code and new password. Log in again afterwards. |
| `auth status` | Checks local/environment credentials, not whether the server still accepts them. Returns an error when logged out. |
| `auth logout` | Deletes the selected credential file. Does not revoke server credentials or clear environment variables. |

Registration also accepts `--current-weight-kg NUMBER`, `--goal-weight-kg NUMBER`, `--height-cm NUMBER`, and `--first-name TEXT`. Country defaults to `US`. Registration does not log you in automatically; follow any account confirmation steps and then run `auth login`.

There is no password flag, password environment variable, or stdin password mode. Run password prompts in a real terminal. Google login uses a short-lived Google ID token; only the resulting FatSecret credentials are retained.

### Foods

| Command | Behavior and defaults |
| --- | --- |
| `foods search QUERY [--page 0] [--size 20] [--market CODE] [--lang CODE]` | Search foods. Pages are zero-based. Quote multiword queries. The market index scopes results (`RU` finds Russian foods; default `US` from config, override with `FATSECRET_MARKET_LOCALE`). |
| `foods get ID` | Food details, including portions when the server supplies them. |
| `foods popular ID [ID ...]` | Popular serving sizes. Use `--format json`; plain output is empty. |
| `foods barcode GTIN` | Look up barcode digits, then fetch the matching food. Preserve leading zeros. |
| `foods preference-types` | List dietary-preference/allergen types. |
| `foods vote-preference ID --vote TYPE=VALUE [--vote TYPE=VALUE ...]` | Submit preference votes. `TYPE` must be numeric; values are passed to the server as strings. |

Get vote types from `foods preference-types`; do not assume a boolean or numeric value vocabulary. A barcode without a recognized food ID prints `NOT_FOUND` in plain mode and still exits successfully. JSON contains food details for a match, or the scan response when no match is recognized.

### Recipes

| Command | Behavior and defaults |
| --- | --- |
| `recipes search QUERY [--page 0]` | Search the legacy recipe index. |
| `recipes get ID` | Recipe details through the same details endpoint used for foods. |
| `recipes categories` | List recipe categories. |
| `recipes cookbook-search QUERY [--page 0] [--size 20]` | Search the cookbook index. |
| `recipes cookbook-count [--market US]` | Recipe count for a market, for example `US`. |

Search pages are zero-based. `--market` applies only to cookbook count; it is not a global search-language setting.

### Food diary

| Command | Behavior and defaults |
| --- | --- |
| `diary day [--date YYYY-MM-DD]` | Read any day's entries with IDs, macros, servings, and day totals. History works (same page + day selector). |
| `diary add --food-id ID --serving-id ID [--meal other] [--meal-id ID] [--date YYYY-MM-DD] [--units 1]` | Add a food using the selected serving. Prefer an explicit date. |
| `diary rm ENTRY_ID [--date YYYY-MM-DD]` | Delete an entry ID shown by `diary day`. `--date` is the recorded date (default today). No confirmation prompt. |
| `diary cp --from YYYY-MM-DD [--date YYYY-MM-DD]` | Copy a day's entries onto another date (same meals, fresh IDs). |

`--units` is a serving multiplier, not grams. Use a gram-based portion if you want to enter a mass. Meal names are case-insensitive:

| Meal | Server ID |
| --- | --- |
| `other` | 0 |
| `breakfast` | 1 |
| `lunch` | 2 |
| `dinner` | 3 |
| `snack` | 4 |
| `prebreakfast` or `pre-breakfast` | 5 |
| `secondbreakfast` or `second-breakfast` | 6 |

`--meal-id` overrides the name. Food ID, serving ID, and diary entry ID are not interchangeable. `diary day` lists entry rows with their IDs, grouped by meal with energy subtotals.

### Weight, exercise, and water

| Command | Behavior and defaults |
| --- | --- |
| `weight log KG [--goal-kg KG]` | Record weight in kilograms; goal behavior is described in the quick start. |
| `exercise day [--date YYYY-MM-DD]` | Read the exercise day (history supported). |
| `exercise types` | List activity type IDs. |
| `exercise log --type-id ID --mins MINUTES [--kcal NUMBER] [--description TEXT]` | Submit activity duration, optional energy, and a note. Without `--kcal`, the request leaves estimation to the server. Verified live. |
| `water day` | Read current-day water data. Requires Premium; non-premium accounts get 299 `Could not save`. |
| `water log ML [--goal-ml 2000]` | Submit an intake increment in milliliters and a daily goal. Requires Premium; non-premium accounts get 299 `Could not save`. |

These commands have no date selector. Water uses the UTC calendar day. Do not repeatedly submit a write just because the human output is sparse; check account state first.

### Saved meals

Saved meals are reusable food collections, distinct from the breakfast/lunch/dinner categories in the diary. Writes go as form POSTs like the app; create/delete round-trip verified live.

| Command | Behavior and defaults |
| --- | --- |
| `meals ls [--meal 0]` | List saved meals using a numeric meal-tab ordinal filter. |
| `meals show ID` | Show a saved meal and its items. |
| `meals quickpicks` | Read quick picks. |
| `meals create --title TEXT [--description TEXT] [--meal-types 1]` | Create a saved meal. Description defaults to empty. |
| `meals save ID --title TEXT [--description TEXT] [--meal-types 1]` | Save a meal's metadata. Omitted description/types use the defaults rather than preserving existing values. |
| `meals rm ID` | Delete a saved meal. |
| `meals log ID [--meal other] [--meal-id ID]` | Add a saved meal to the diary; uses the named-meal mapping above. |
| `meals add-item --meal-id ID --food-id ID --name TEXT --portion-id ID [--units 1] [--item-id 0]` | Add an item; use an existing `--item-id` to edit it. |
| `meals rm-item ITEM_ID` | Delete a saved-meal item. |

`meals ls --meal` is a tab ordinal, `--meal-types` is the server's mask string, and `meals add-item --meal-id` identifies a saved meal. These are not interchangeable with the diary meal ID.

### Account and other reads

| Command | Behavior |
| --- | --- |
| `account show` | Read account details, including weight-related fields when supplied. |
| `account settings` | Read the legacy account settings page. |
| `account change-username NAME` | Change the member name immediately if accepted; no confirmation prompt. |
| `settings attributes` | Read attributes such as birth date, gender, and names. |
| `settings locale` | Read language/market settings; does not change them. |
| `notifications ls` | Read notifications. |
| `feed blocks` | Read the block list; see the direction caveat in current coverage. |
| `feed blocking` | Read the other block-related list; see the same caveat. |
| `learning progress` | Read progress and bookmarks. |
| `learning content` | Read content for the account's language. |
| `food-groups` | Read food group data. |

Use `--format json` for nested settings, notifications, and learning responses when the human view omits fields.

## Output and scripting

Data commands write results to stdout. Errors and tracing diagnostics go to stderr. A successful invocation exits with code `0`; runtime errors are nonzero, and invalid CLI arguments normally exit with code `2`.

- `human` is the default: tables, summaries, or a hint to inspect JSON.
- `json` prints the parsed API value as pretty JSON. Legacy XML is converted to JSON; text nodes can appear as `{"$text":"..."}` and singleton objects may become arrays when repeated. Empty successful responses can be `null`.
- `plain` prints command-specific, usually tab-separated data without a shared schema. Writes commonly print `OK`. Some commands have no useful plain representation.

Authentication commands use `--format` (`human`/`plain` text, or JSON status objects). Completions always print shell code. JSON reflects remote response shapes, not a versioned CLI schema; handle absent fields and avoid assuming a fixed structure across endpoints.

For example, with [jq](https://jqlang.org/) installed:

```sh
fatsecret-cli foods search "oatmeal" --format json \
  | jq '.recipes[]? | {id, title}'

fatsecret-cli foods search "oatmeal" --format plain
fatsecret-cli learning progress --format json
```

In Bash or Zsh scripts, enable `set -o pipefail` if a downstream command must not hide a CLI failure. Barcode `NOT_FOUND` is an explicit exception to treating exit code `0` as a match. A successful write response also does not prove that an entry appears on the expected date; verify in the app.

## Configuration

No configuration file is required for normal use. The CLI resolves configuration values in this order:

1. `FATSECRET_*` environment overrides.
2. The selected TOML file.
3. Built-in defaults.

File selection is separate: `--config PATH` takes precedence over `FATSECRET_CONFIG`, then the platform default path. A missing default file is fine; a missing explicitly selected file is an error.

The default directory contains `config.toml`, `credentials.json`, and `device.json`:

| Platform | Default directory |
| --- | --- |
| macOS | `~/Library/Application Support/fatsecret-cli/` |
| Linux | `$XDG_CONFIG_HOME/fatsecret-cli/`, or `~/.config/fatsecret-cli/` when unset |
| Windows | The user's roaming application-data directory, normally `%APPDATA%\fatsecret-cli\` |

A minimal optional configuration:

```toml
device_model = "android"
app_version = "11.8.0.5"
```

```sh
fatsecret-cli --config ./config.toml foods search "oatmeal"
```

Every supported TOML key has an environment override:

| TOML key | Environment variable | Purpose |
| --- | --- | --- |
| `device_model` | `FATSECRET_DEVICE_MODEL` | Device model string; must not be empty. |
| `app_version` | `FATSECRET_APP_VERSION` | Mobile app version sent in headers. |
| `device_id` | `FATSECRET_DEVICE_ID` | Override the installation identity. Normally leave unset. |
| `auth_url` | `FATSECRET_AUTH_URL` | Login endpoint. |
| `google_auth_url` | `FATSECRET_GOOGLE_AUTH_URL` | Google login endpoint (HTTPS; loopback HTTP allowed for tests). |
| `food_search_url` | `FATSECRET_FOOD_SEARCH_URL` | Food search. |
| `food_popular_url` | `FATSECRET_FOOD_POPULAR_URL` | Popular servings. |
| `food_vote_url` | `FATSECRET_FOOD_VOTE_URL` | Dietary-preference votes. |
| `food_types_url` | `FATSECRET_FOOD_TYPES_URL` | Dietary-preference types. |
| `recipe_count_url` | `FATSECRET_RECIPE_COUNT_URL` | Cookbook count. |
| `scan_url` | `FATSECRET_SCAN_URL` | Barcode scan. |
| `journal_url` | `FATSECRET_JOURNAL_URL` | Food diary writes. |
| `register_url` | `FATSECRET_REGISTER_URL` | Account registration. |
| `forgot_url` | `FATSECRET_FORGOT_URL` | Password-reset email. |
| `reset_url` | `FATSECRET_RESET_URL` | Password-reset completion. |
| `user_details_url` | `FATSECRET_USER_DETAILS_URL` | Account details. |
| `change_username_url` | `FATSECRET_CHANGE_USERNAME_URL` | Account rename. |
| `feed_add_block_url` | `FATSECRET_FEED_ADD_BLOCK_URL` | Feed user block. |
| `learning_course_bookmark_save_url` | `FATSECRET_LEARNING_COURSE_BOOKMARK_SAVE_URL` | Course bookmark save. |
| `learning_course_bookmark_delete_url` | `FATSECRET_LEARNING_COURSE_BOOKMARK_DELETE_URL` | Course bookmark delete. |
| `learning_lesson_bookmark_save_url` | `FATSECRET_LEARNING_LESSON_BOOKMARK_SAVE_URL` | Lesson bookmark save. |
| `learning_lesson_bookmark_delete_url` | `FATSECRET_LEARNING_LESSON_BOOKMARK_DELETE_URL` | Lesson bookmark delete. |
| `learning_lesson_progress_save_url` | `FATSECRET_LEARNING_LESSON_PROGRESS_SAVE_URL` | Lesson progress save. |
| `market_locale` | `FATSECRET_MARKET_LOCALE` | Food-search market index (`RU` finds Russian foods). |
| `language_locale` | `FATSECRET_LANGUAGE_LOCALE` | UI language sent to modern indexes. |
| `server_base` | `FATSECRET_SERVER_BASE` | Legacy endpoint base; include a trailing slash. |

Manage the file without editing TOML by hand:

```sh
fatsecret-cli config show
fatsecret-cli config set market_locale RU
fatsecret-cli config unset market_locale
fatsecret-cli config path
```

`config show` masks the device id. Values set with `config set` beat built-in
defaults but lose to `FATSECRET_*` environment variables.

Default modern endpoints are under `https://app.ftscrt.com/`; the legacy base is `https://android.fatsecret.com/android/`. Attributes, block-list reads, and learning reads use fixed modern URLs with no endpoint override. Changing `server_base` does not redirect every request.

Changing `--config` does not relocate credentials or device identity and does not select a separate account profile.

## Credentials and privacy

The CLI stores server-issued credentials in `credentials.json`, not your password. This file is unencrypted and is not backed by the OS keychain. Unix writes set file mode `0600`; anyone who can read the file can obtain its credentials. Protect the directory, backups, and host account. On Windows, set suitable ACLs yourself.

For automation, the following environment variables override credential-file loading when all three required values are present:

| Variable | Purpose |
| --- | --- |
| `FATSECRET_SERVER_ID` | Numeric server account ID. |
| `FATSECRET_SECRET_KEY` | Secret account credential. |
| `FATSECRET_DEVICE_KEY` | Server-issued device credential. |
| `FATSECRET_USERNAME` | Optional name displayed by `auth status` with environment credentials. |
| `FATSECRET_CREDENTIALS` | Override the credential-file path for reading, saving, and logout. |

A partial environment triple falls back to the file; it does not merge field by field. Inject secrets through your automation system's secret storage rather than typing them into shell history. There is no built-in credential export command. Never commit credential files or plaintext `.env` files, or include their contents in issue reports.

`device.json` stores a separate Firebase installation identity. Login, registration, and password recovery resolve an explicit identity first, then reuse the saved identity, or register a new one with Firebase. `FATSECRET_DEVICE_ID` is not the same as `FATSECRET_DEVICE_KEY`.

`auth logout` deletes only the selected credential file. It leaves `device.json`, configuration, and any environment credentials intact. To stop using environment credentials, clear them in the parent shell or automation environment. Deleting a local file does not revoke credentials on the server.

Account, diary, notifications, and learning output may contain personal data. Review JSON and diagnostics before sharing them. There is no dry-run mode or general confirmation prompt for writes and deletes.

## Shell completions

Supported shells: Bash, Elvish, Fish, PowerShell, and Zsh.

```sh
fatsecret-cli completions bash
fatsecret-cli completions elvish
fatsecret-cli completions fish
fatsecret-cli completions powershell
fatsecret-cli completions zsh
```

Each command writes a completion script to stdout without accessing your account. To load it for the current Bash session:

```bash
source <(fatsecret-cli completions bash)
```

For Fish, save it in the user completion directory:

```fish
mkdir -p ~/.config/fish/completions
fatsecret-cli completions fish > ~/.config/fish/completions/fatsecret-cli.fish
```

For Zsh, place the generated file named `_fatsecret-cli` in a directory on `fpath` and initialize completions with `compinit`. Follow your shell's usual startup-file setup for persistent loading.

## Troubleshooting

### The executable is not found

Ensure Cargo's binary directory is on `PATH`, or use `./target/release/fatsecret-cli` after a release build. A debug build is at `./target/debug/fatsecret-cli`.

### Password prompt fails in a pipe or CI

The prompt requires an interactive terminal and may report `Device not configured` without one. Do not pipe a password or add a password argument. Log in interactively, or provision the server-issued credential triple through secure automation storage.

### Logged out or credentials are rejected

`auth status` checks whether credentials can be loaded locally; it does not contact FatSecret. Run `auth login` again if the server rejects them. If environment credentials are set, they continue to override a newly written credential file. Check variable presence without printing secret values.

### Login returns type 299

A missing installation identity caused this generic login error in earlier live checks. The client now provisions one automatically. Check Firebase connectivity and any explicit `FATSECRET_DEVICE_ID` override. The error is generic; it does not by itself prove that device identity is the current cause.

### Diary date or entry IDs are wrong or missing

Default diary dates are the current UTC calendar day (floored Unix epoch days). That matches `recordedDate`/`dateInt`. Local timezone is not used. Pass `--date YYYY-MM-DD` on `diary add` and `diary rm` when you want a specific civil date.
`diary day` reads any server day: pass `--date YYYY-MM-DD` for history (same page + day selector). The page is read as a form POST like the app; a GET returns only the `{dateint, guid}` shell.

Date parsing rejects impossible calendar dates (including February 29 on non-leap years) but stays lenient on padding and extra segments. Supply a real `YYYY-MM-DD` date.

### `Unable to save` or an empty rejection response

Saved-meal writes, exercise logging, and water operations have known live failures. Legacy endpoints can return HTTP 200 with `Unable to save`; the CLI treats that as an error. HTTP success alone is not evidence of a saved change. Do not repeatedly retry mutations without checking account state.

### Missing fields, unusual units, or sparse tables

Use `--format json` to inspect the parsed response. Legacy XML has different shapes from modern JSON, and human renderers show only selected fields. Requests send `unit: kj`, but live food values can match kilocalorie figures (oats `energyPerPortion` 389 per 100 g). Human tables label the number as `energy` and do not convert it. Check the source field and the mobile app before interpreting energy values.

The CLI has no global locale or unit-selection flag. `settings locale` only reads settings. Do not assume all food queries use the market supplied to `recipes cookbook-count`.

### Logging

When `RUST_LOG` is unset, `-v` and `-vv` set tracing to `info` and `debug`. `--color never` disables log ANSI on stderr. Avoid broad debug/trace logging when handling credentials or personal data, and redact diagnostics before sharing them.

## Development

```sh
cargo build --locked
cargo test --locked
cargo fmt --check
cargo clippy --all-targets --locked -- -D warnings
```

Tests cover local behavior and CLI help; passing them does not verify remote endpoint compatibility. Use a test account for live writes and check the resulting state in FatSecret. Do not add live secrets to fixtures.

| Path | Responsibility |
| --- | --- |
| `src/cli.rs` | Commands, flags, and help. |
| `src/commands/` | Command dispatch and workflows. |
| `src/api/client.rs` | Requests, endpoint handling, JSON/XML parsing. |
| `src/auth/` | Login, credential storage, installation identity. |
| `src/config.rs` | TOML, environment overrides, defaults. |
| `src/output.rs` | Human, plain, and JSON output. |
| `src/main.rs` | CLI startup, tracing, completions, error reporting. |
| `tests/cli_help.rs` | CLI help and unauthenticated-access checks. |

When changing a command, keep its help and both README versions consistent. For bug reports, include the CLI version, OS, command with sensitive arguments removed, exit status, and a redacted error or response. State whether the corresponding action works in the mobile app. Do not include passwords, credential files, secret headers, or private account data.

## License

`Cargo.toml` declares `MIT OR Apache-2.0`. This checkout does not include standalone license text files. FatSecret names and service data belong to their respective owners; the package's license declaration does not grant rights to those services or data.

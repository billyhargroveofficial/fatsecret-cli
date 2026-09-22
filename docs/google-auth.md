# Google authentication

`fatsecret-cli auth google` uses the Google button on the real FatSecret sign-in page, exchanges its Google ID token for a mobile session, and stores the same credential triple used by password login. Normal commands remain direct HTTP requests; they start no browser, emulator, timer or daemon.

## Use

Install [Playwriter](https://github.com/remorses/playwriter) and connect it to an existing Chrome. The CLI calls `playwriter session new` without browser-launch flags. On a remote desktop, complete sign-in in that desktop's Chrome. Passwords and two-factor codes belong in Google's browser UI, never in the terminal or chat.

```sh
fatsecret-cli auth google
fatsecret-cli auth status
fatsecret-cli foods search "гречка" --market RU --lang ru --format json
fatsecret-cli diary day --date "$(date +%F)" --format json
```

Use your existing Google-linked FatSecret account. The CLI requires `isLinked: true` and a complete mobile credential triple before replacing local credentials. It does not call the account registration endpoint. Apple and Facebook sign-in are not implemented.

The default interactive timeout is 600 seconds; `--timeout SECONDS` accepts 1–3600. Closing the FatSecret tab, Ctrl-C, SIGTERM, a failed browser step or the timeout triggers cleanup of owned tabs and the Playwriter session. An unreachable browser can prevent cleanup; the command reports that situation. Force-killing the process cannot run cleanup.

`FATSECRET_PLAYWRITER` may point to an alternate local Playwriter executable. The helper tracks the exact target it creates and its own OAuth popups. Other browser tabs and the shared browser process are left running.

For integrations that already obtain a Google ID token for FatSecret, `auth google --token-stdin` accepts a pipe. There is deliberately no token argument or token environment variable. Do not paste real tokens into shell commands, issue reports or logs. A normal Google token for a different application's client ID will not work.

## Observed protocol

Independently inspected in FatSecret Android **11.8.0.6 (805)** on 2026-09-22:

- Resource `path_link_with_google`: `https://app.ftscrt.com/api/authenticate/v2/google`.
- `LinkUserWithGoogleDTO` serializes `token`, `checkUserExists`, `deviceIdentifier`.
- `checkUserExists: false` is the sign-in request. **`true` is not an existing-account-only guard**: a live request against an existing account returns HTTP 400/type 3, “This social account already exists, please try and sign in instead.”
- Response fields: `isLinked`, `serverId`, `deviceKey`, `secretKey`, `userName`, `email`.
- Android `server_client_id_google` and the Google Identity Services button at `https://foods.fatsecret.com/Auth.aspx?pa=s` use the same public client ID: `114255563282-i78p7sf0gccbup57kaqg8420fge24p9u.apps.googleusercontent.com`.
- The website submits the credential in an ASP.NET form field ending in `$GoogleCode`. The helper intercepts only that POST to the exact FatSecret origin/path and passes its credential to the mobile exchange. It does not send the website POST or switch the existing website session.

If Chrome is already signed into the FatSecret website, it redirects the login page to the home page. The helper fetches the original public login document with browser `fetch(..., {credentials: "omit"})` and serves that document only in its owned tab. Chrome's configured network routing stays in use. It does not clear cookies, sign out, create another profile or replace Google's OAuth client.

The inspected base APK SHA-256 was `9f4d8e162dde7d5f6cf104555d60d8be965019e440faebce713d1cc89fa5cbac`. APKs and decompiled sources are not included in this repository.

The private mobile API can change. Google exchanges use a 15-second connect timeout, 45-second request timeout, HTTPS (loopback HTTP for tests), and no HTTP redirects. FatSecret verifies the Google token; the CLI's JWT syntax check is not a signature verifier.

## Credential handling and validation

The browser credential is briefly passed through a 0600 file in a private temporary directory and removed when browser capture finishes. The stored `credentials.json` contains only the FatSecret session, with atomic replacement and 0600 permissions on Unix. Failed login leaves an existing credential file intact. CLI output contains the username and status, never the credential triple or Google token. Raw server errors and Playwriter output are not relayed because they may contain private data.

`cargo test --locked` includes local-server checks for the exact request, successful storage, malformed/unlinked/rejected responses, preserving old credentials, no token echo, insecure URL and redirect rejection, and browser cleanup on success, failure, timeout and SIGTERM. Live validation on 2026-09-22 completed the full `auth google` browser flow with an already signed-in website session, Russian food search, and a read of the existing cloud diary. No diary entries were written during validation.

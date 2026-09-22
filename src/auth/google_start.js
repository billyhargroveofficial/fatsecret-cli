// Executed by Playwriter, using its session-scoped state and the existing Chrome.
// tokenPath is supplied by Rust in a private temporary directory.
const fs = require("node:fs");
const path = require("node:path");
state.ownedPages = [];
state.ownedTargetIds = [];
state.closing = false;
state.trackOwned = (owned) => {
  state.ownedPages.push(owned);
  state.ownedTargetIds.push(owned.targetId());
  owned.on("popup", state.trackOwned);
  if (state.closing) owned.close().catch(() => {});
};
const cdp = await getCDPSession({ page });
const { targetId } = await cdp.send("Target.createTarget", {
  url: "about:blank", background: true,
});
state.ownedTargetId = targetId;
state.page = context.pages().find(p => p.targetId() === targetId)
  || await context.waitForEvent("page", { predicate: p => p.targetId() === targetId });
state.trackOwned(state.page);
await state.page.route(url => url.origin === "https://foods.fatsecret.com"
  && url.pathname.toLowerCase() === "/auth.aspx", async route => {
  const request = route.request();
  const url = new URL(request.url());
  if (url.origin !== "https://foods.fatsecret.com"
      || url.pathname.toLowerCase() !== "/auth.aspx" || request.method() !== "POST") {
    await route.continue();
    return;
  }
  const form = new URLSearchParams(request.postData() || "");
  for (const [key, value] of form) {
    if (!key.endsWith("$GoogleCode") || !value) continue;
    // Never print request bodies or the credential. Publish atomically so Rust
    // cannot read a half-written token; all files are inside its 0700 directory.
    if (!fs.existsSync(tokenPath)) {
      fs.writeFileSync(tokenPath + ".pending", value, { mode: 0o600, flag: "wx" });
      fs.renameSync(tokenPath + ".pending", tokenPath);
    }
    // The mobile exchange happens in Rust. Do not switch the shared browser's
    // website account or create website cookies as a side effect of CLI login.
    await route.fulfill({ status: 200, contentType: "text/html", body: "Google sign-in received. You can return to the terminal." });
    return;
  }
  await route.continue();
});
state.page.on("close", () => {
  if (!state.closing) fs.writeFileSync(path.join(path.dirname(tokenPath), "closed"), "", { mode: 0o600 });
});
await state.page.goto("https://foods.fatsecret.com/Auth.aspx?pa=s", { waitUntil: "domcontentloaded" });
if (!(await state.page.locator("#g_id_onload").count())) {
  // FatSecret redirects an already signed-in browser to its home page. Fetch
  // the actual public login document without credentials, through Chrome's
  // existing network route. Display it only in our tab; no cookie clearing,
  // sign-out, new browser profile or custom Google client is needed.
  const publicLogin = await state.page.evaluate(async () => {
    const response = await fetch("https://foods.fatsecret.com/Auth.aspx?pa=s", { credentials: "omit" });
    if (!response.ok) throw new Error("FatSecret public login page unavailable");
    return response.text();
  });
  if (!publicLogin.includes('id="g_id_onload"')) throw new Error("FatSecret Google button was not found");
  await state.page.route("https://foods.fatsecret.com/Auth.aspx?pa=s", route =>
    route.fulfill({ status: 200, contentType: "text/html; charset=utf-8", body: publicLogin }), { times: 1 });
  await state.page.goto("https://foods.fatsecret.com/Auth.aspx?pa=s", { waitUntil: "domcontentloaded" });
}
// The user completes authentication in the real site's Google UI. No fake
// sign-in page, password handling, cookie export or injected OAuth client.
await state.page.bringToFront();
await state.page.frameLocator("iframe[title*=Google]").getByRole("button").click({ timeout: 15000 });

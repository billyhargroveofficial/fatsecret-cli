state.closing = true;
let failed = false;
for (const owned of state.ownedPages || []) {
  try {
    if (!owned.isClosed()) await owned.close();
  } catch {
    failed = true;
  }
}
// If matching the newly created Page failed, close that precise CDP target.
if (state.ownedTargetId && (!state.ownedPages || state.ownedPages.length === 0)) {
  const cdp = await getCDPSession({ page });
  await cdp.send("Target.closeTarget", { targetId: state.ownedTargetId });
}
state.ownedPages = [];
state.ownedTargetIds = [];
state.page = undefined;
if (failed) throw new Error("Could not close every owned sign-in tab");

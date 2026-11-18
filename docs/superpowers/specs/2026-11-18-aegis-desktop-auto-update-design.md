# Aegis Desktop Auto-Update — Design

Date: 2026-11-18
Status: draft for approval
Scope: `apps/desktop/aegis-desktop` (crate `aegis-desktop`, package `aegis-desktop`)

---

## 1. Intent and the why

`aegis-desktop` currently ships the Tauri updater plugin but drives none of
it. `src-tauri/Cargo.toml` depends on `tauri-plugin-updater`, `package.json`
depends on `@tauri-apps/plugin-updater`, `src-tauri/src/lib.rs` registers
`tauri_plugin_updater::Builder::new().build()`, and `src-tauri/tauri.conf.json`
declares the endpoint, the signing pubkey, `dangerousInsecureTransportProtocol`,
and the passive Windows install mode. What is missing is the orchestration:
nothing checks the endpoint, nothing downloads, nothing tells the user an
update exists, and nothing installs it.

The goal is a release flow a user never has to think about: the app checks
quietly, downloads the artifact quietly, and only interrupts when there is
something to install. By the time the user is asked, the installer is already
on disk, so accepting is a single click that takes seconds rather than
minutes.

This is a desktop app used in a regulated clinical-research environment
(CDISC SDTM terminology, CRF authoring). Users work in long sessions against
trial data. An update that interrupts mid-task is worse than an update that
arrives at a convenient moment — which is the entire reason the flow is
"download first, ask later" rather than "ask, then download".

## 2. Goals

1. Check for a new release at launch, and once every hour after that.
2. Never block or degrade the UI while checking or downloading.
3. Download **and verify** the release artifact in the background *before*
   asking the user anything.
4. Tell the user once that an update is ready, and offer "Update now" or
   "Later".
5. After the user defers, keep the update discoverable in Settings rather
   than nagging, and do not re-download the same artifact on every launch.
6. Install and restart the app on Windows.

## 3. Non-goals

- **macOS and Linux install.** The plugin's macOS/Linux path launches the
  installer but does not relaunch the app; that needs a separate relaunch
  mechanism and a different artifact set. The product target is Windows; the
  code is written so those platforms degrade to "update available" without
  breaking, but no relaunch is implemented for them.
- **Rollback, staged rollout, or release channels.** One endpoint, one
  "newest version wins" rule, straight from the plugin's default comparator.
- **Server-side release metadata.** The release manifest is served by the
  existing release server, not by `aegis-server`. The desktop does not gain
  a release endpoint.
- **Automatic install without confirmation.** Explicitly contrary to the
  request.
- **Delta or partial downloads.** The plugin downloads whole artifacts.

## 4. Anti-goals

Things that would make this a failure even though it "works":

- More than one *scheduled* release check per hour, per install. (A
  user-initiated check from the Settings page is an explicit exemption.)
- Downloading the same version twice within a session.
- Re-prompting a user who already deferred the same version.
- A check loop that runs once per open window.
- Re-downloading an artifact the machine already has, on every launch.
- A UI that blocks, stutters, or loses the user's place because of an update.

## 5. Constraints

These are properties of the existing codebase and the plugin, discovered
during design. They constrain the solution rather than being negotiable.

| Constraint | Consequence |
|---|---|
| `src/main.tsx` is the entry point for the main window **and** every `project:<code>` workspace window (`api.openProjectWorkspace` opens a `WebviewWindow` on the same bundle). | A frontend-owned timer would check and download once per open window. The background loop must live in the Rust process. |
| The plugin's Windows `install_inner` shells out to the NSIS installer with `/P /UPDATE /R` and then calls `std::process::exit(0)`; the installer relaunches the app. | "Install and restart" needs no extra restart code on Windows. It also means the frontend's install call never resolves, so nothing may depend on it completing. |
| The plugin's Rust API is `UpdaterExt::updater()` → `check()` → `Update::download()` → `Update::install(bytes)`. The JS API is a separate path over the same plugin. | Using the Rust API for the loop and the JS API for the UI would mean two independent checks. We use the Rust API for the loop; the frontend never calls the updater plugin directly. |
| `app.updater()` reads the plugin's `Config` straight out of `tauri.conf.json`'s `plugins.updater` block (the plugin clones `api.config()` at setup and only overrides what `Builder::new()` was given, which is nothing here). | The Rust path picks up the existing endpoint, pubkey, `dangerousInsecureTransportProtocol` and `windows.installMode` with no extra wiring. There is no second place to configure the release server. |
| `tauri.conf.json` sets `app.windows[0].visible = false`; the window is shown by `main.tsx` ~150 ms after mount. `setup()` runs before any webview exists. | An event emitted during `setup()` is emitted before the frontend can subscribe. The frontend must not depend on catching a "ready" event to know the current status. |
| The release manifest is deserialized by the plugin into `RemoteRelease`, which requires `platforms` (a map keyed by target) or a top-level `url` + `signature`; `version` must be greater than the running app's version for the default comparator to offer an update. | See §7.12 — the currently-served manifest satisfies neither. |
| `docs/guidelines/aegis-desktop-development.md` states the Rust side is transport-only, proxying `aegis-server`. | The updater talks to the release server, not `aegis-server`, so it is **not** a `http/` module. It gets its own module and its own commands; `http/` is untouched. |
| The desktop Rust crate reuses `http::dto::ApiError` as the single error type for every command. | The updater commands return `ApiError` too, so the frontend's `toApiError` / `errorMessage` helpers work unchanged. |
| The global `QueryClient` defaults are `staleTime: Infinity`, `retry: false`, `refetchOnWindowFocus: false`, `refetchOnReconnect: false`. | A status query is never refetched on its own. Anything that must change while the app is open needs an explicit trigger — a `refetchInterval` that is only active while a download runs, or an event-driven invalidation. |

## 6. Approaches considered

### A — Rust owns the loop, TypeScript draws the dialog *(chosen)*

A new `src-tauri/src/updater/` module holds the policy: a state machine, a
source trait, and a service that owns the worker task. `setup()` spawns one
worker. The frontend adds a `features/updater/` module that reads the status
and renders a dialog and a Settings row.

- One check loop per process, so one check per install regardless of how many
  windows are open.
- The verified artifact lives in Rust-managed state and on disk, so "Update
  now" is instant and does not depend on a webview still being alive.
- Mirrors the house pattern: `logging_utils::LogSubmitter` is already
  "config + interval + worker + managed state".
- The `UpdateSource` seam lets the state machine be tested without a network.

### B — TypeScript owns the loop

A hook in `main.tsx` calls the plugin's JS `check()` → `update.download()`,
then `setInterval`s hourly. The bytes stay in the plugin's Rust-side resource
behind the JS handle.

- *Pro:* much less code; all in TypeScript; easily covered by vitest by
  mocking `@tauri-apps/plugin-updater`; uses the plugin's documented JS path.
- *Con:* the timer is a React effect, so it dies with the webview and
  restarts on reload; it needs the same main-window gate that every future
  window type must remember to honour; the JS handle has to live in a
  module-level variable so a different route can install the already
  downloaded bytes.

### Dropped: Rust checks, TypeScript downloads

Splits one job across two runtimes, hits the release endpoint twice, and
still requires the frontend to be alive to perform the download — which
reintroduces the per-window problem it was meant to avoid. Buys nothing over A.

### Why A over B

The deciding factor is the multi-window entry point. `main.tsx` runs in every
window, so B's correctness depends on a label gate that every future window
type must remember. A is correct by construction. A's second benefit is that
"download first, then notify" becomes a real guarantee rather than a
best-effort: the artifact is in Rust state before the user is ever told.

## 7. Design

### 7.1 Rust module layout

```
src-tauri/src/updater.rs           ← barrel: pub mod status; pub mod source; pub mod service;
src-tauri/src/updater/status.rs    ← UpdateStatus + its DTO form
src-tauri/src/updater/source.rs    ← UpdateSource / PendingRelease traits + plugin impl
src-tauri/src/updater/service.rs   ← UpdateService: worker, shared status, cache, install
src-tauri/src/commands/updater.rs  ← #[tauri::command] shims
```

`commands/updater.rs` keeps the guideline's naming rule (a feature with a
Rust twin uses the same kebab-case name on both sides:
`features/updater/` ↔ `commands/updater.rs`). It is the one command module
with no `http/` twin, because the release server is not `aegis-server`.

### 7.2 The state machine

```rust
pub enum UpdateStatus {
    Idle,
    Checking,
    Downloading { version: String, downloaded: u64, total: Option<u64> },
    Ready { version: String, notes: Option<String>, date: Option<String> },
    Installing { version: String },
    Failed { message: String },
}
```

Transitions, all owned by `UpdateService`:

```
Idle --check()--> Checking
Checking --no update--> Idle
Checking --error--> Failed
Checking --release found--> Downloading   (or straight to Ready, from the disk cache)
Downloading --error--> Failed
Downloading --bytes verified--> Ready   (emits `aegis:update-ready`)
Ready --install()--> Installing
```

`Installing` is effectively terminal on Windows — `install` exits the process
before the command returns — but it is kept as a real state so the
macOS/Linux path, where the installer is launched but the app is not
restarted, can report "installed, please restart" rather than falling back
to `Ready`.

**`Ready` means downloaded *and verified*.** The plugin does not do this for
us: `Update::download` returns raw bytes and performs no signature check —
verification happens inside `Update::install`. Left alone, a tampered or
corrupt artifact would only be caught after the user clicked "Update now".
`Update::verify(&self, bytes)` is public, so the service calls it explicitly
after the download and only transitions to `Ready` once it passes. A
verification failure is a `Failed` status, not a `Ready` one.

Two rules make the flow non-repeating:

- **No re-download.** The worker skips a release whose version is **not newer
  than** the one it already holds. Comparing with "not newer" rather than
  "different" matters: the plugin's comparator only offers releases newer
  than the *installed* version, so a server-side rollback to a version that
  is still above the installed one (installed 0.1.0, holding 0.2.0, server
  announces 0.1.5) would otherwise be downloaded and would fire a second
  prompt — a second dialog for a downgrade. Comparing versions needs semver
  ordering, so `semver` is added as a dependency of the desktop crate.
- **One prompt per version per session.** The worker emits
  `aegis:update-ready` exactly once per version, because the status only
  leaves `Ready` on install.

### 7.3 The source trait

The policy code never touches `tauri_plugin_updater` directly, so the state
machine can be tested offline:

```rust
#[async_trait]
pub trait UpdateSource: Send + Sync {
    async fn check(&self) -> Result<Option<Arc<dyn PendingRelease>>, UpdateError>;
}

#[async_trait]
pub trait PendingRelease: Send + Sync {
    fn version(&self) -> &str;
    fn notes(&self) -> Option<&str>;
    fn date(&self) -> Option<&str>;
    async fn download(&self, on_progress: &(dyn Fn(u64, Option<u64>) + Send + Sync))
        -> Result<Vec<u8>, UpdateError>;
    /// Checks the package signature against the configured pubkey.
    fn verify(&self, bytes: &[u8]) -> Result<(), UpdateError>;
    fn install(&self, bytes: &[u8]) -> Result<(), UpdateError>;
}
```

`PluginUpdateSource` implements `UpdateSource` with
`app.updater()?.check().await?`; `PluginRelease` wraps the plugin's `Update`
and implements `PendingRelease` with `update.download(...)`,
`update.verify(...)` and `update.install(bytes)`.

`install` and `verify` stay on the trait object rather than being reduced to
"bytes plus a version" because the plugin's `Update::install` and
`Update::verify` need `self` to reach the plugin's config and the release's
signature. The bytes alone are neither installable nor verifiable.

The plugin's `Update` is `Send + Sync` — every field of its private
`UpdaterContext` is (`Config` is plain data; the three callbacks are
`Arc<dyn Fn … + Send + Sync>`), and `Update` is `Clone` — so it can be held
in Tauri managed state and shared with the worker task.

The release date is carried as a string. The plugin exposes
`Option<OffsetDateTime>`, and `OffsetDateTime`'s `Display` is RFC 3339, so no
new dependency on `time` is needed.

Tests use a `FakeUpdateSource` whose `FakeRelease` returns canned bytes,
verifies against a known-good digest, and records that `install` was called.

### 7.4 The artifact cache

The artifact is ~6 MB. That is small enough that re-downloading it on every
launch is not fatal, but it is still a silent, pointless transfer for a user
who has deferred the update — every launch, forever. The verified bytes are
therefore cached on disk:

```
<app_data_dir>/updates/<version>/artifact.bin
```

The flow becomes:

1. `check()` returns a release for version V.
2. If `<app_data_dir>/updates/V/artifact.bin` exists, load it and `verify` it
   against the signature in the *current* manifest. If it verifies, go
   straight to `Ready` — no download.
3. If it does not exist, or fails verification (a re-release of the same
   version number, say), delete the stale file, download, verify, and only
   then write it to the cache and transition to `Ready`.

The `Update` object itself is **not** cached. It is always re-fetched by the
network `check()`, which is a small JSON request. Caching it would mean
using the plugin's `Updater::restore_update`, and that path is a trap here:
`restore_update` forces `require_signed_version = true`, and the release
server's current signatures do not satisfy it — see §7.12.

Installing reads the bytes back from the cache. Only the newest version's
directory is kept; older ones are removed when a newer one is cached.

### 7.5 The worker

```rust
// in setup()
let service = UpdateService::new(app_handle.clone(), UpdateConfig::default());
app.manage(service.clone());
tauri::async_runtime::spawn(async move { service.run().await });
```

`UpdateService::new` builds the `PluginUpdateSource` from the app handle and
resolves the cache directory from `app.path().app_data_dir()`. A second
constructor takes an explicit `Arc<dyn UpdateSource>` and cache directory so
tests can inject a fake; production code only ever uses `new`.

`run()` is:

```rust
loop {
    self.check_once().await;
    tokio::time::sleep(self.inner.config.interval).await;
}
```

`UpdateConfig` carries `interval` (default one hour) and mirrors
`LogSubmitterConfig`'s shape.

**Concurrency.** A single check runs at a time. The service holds an
"in flight" flag; `check_once` returns immediately if a check is already
running, and the manual trigger (§7.6) behaves the same way. This is what
stops a user clicking "Check for updates" during a download from starting a
second download of the same bytes.

There is deliberately **no shutdown handshake**. Unlike the log submitter,
which has pending files to drain, a half-finished updater download has
nothing to lose: the partial artifact is discarded and the next launch
re-checks. Adding a drain deadline would be machinery for a case that does
not exist. The worker task is detached and dies with the process.

This requires `tokio`'s `time` feature, which nothing in the graph currently
enables: `tauri` depends on `tokio` with
`["rt", "rt-multi-thread", "sync", "fs", "io-util"]` and not `time`. The
feature is declared **on the desktop crate's own `tokio` dependency**
(`tokio = { workspace = true, features = ["time"] }`), not on the workspace
entry, so the change stays local to the crate that needs it.

`tauri::async_runtime::spawn` is the right entry point here rather than
`tokio::spawn`: it lazily initialises Tauri's global runtime
(`RUNTIME.get_or_init(default_runtime)`), so it works from `setup()`, which
runs outside any tokio runtime context.

### 7.6 Commands

Three commands, all returning `Result<_, ApiError>`:

| Command | Purpose |
|---|---|
| `update_status` | Read the current status. The authoritative source the frontend polls or subscribes to. |
| `check_for_updates` | Trigger a check now. Returns the resulting status. |
| `install_ready_update` | Install the ready artifact. On Windows the process exits inside the call. |

`check_for_updates` is the manual trigger behind the Settings button. It is
an explicit exemption from the "one check per hour" anti-goal — the user
asked for it. It still respects the in-flight guard: if a check or download
is already running, it returns the current status rather than starting a
second one.

`install_ready_update` takes the `Ready` state and calls
`PendingRelease::install(bytes)`. On Windows this launches the NSIS
installer with `/P /UPDATE /R` and exits the process; the installer
relaunches the app. On macOS/Linux it returns after launching the installer
and the user restarts manually.

Because the process exits inside `install`, the frontend's install call never
resolves on Windows. Nothing may depend on it completing — see §7.9.

### 7.7 Status exposure and events

Two channels, for two different needs:

- **`update_status` command → TanStack Query.** The authoritative current
  status. The frontend reads this on mount, so an update that became ready
  *before* the webview subscribed is still seen. This is what makes the
  `setup()`-before-webview ordering safe.
- **`aegis:update-ready` event → the main window.** A signal that the status
  just changed to `Ready`, so an open window can react without polling. It is
  emitted with `Emitter::emit_to("main", ...)`, because the dialog belongs to
  the main window and workspace windows have their own layout.

The frontend's listener invalidates the status query; the dialog's open state
is *derived* from the query result, not from the event payload. That way
there is exactly one source of truth and the two channels cannot disagree.

The status the command returns is not just the phase enum — the Settings row
needs three more facts:

```rust
pub struct UpdateStatusDto {
    pub status: UpdateStatus,
    /// The running app's version, from `app.package_info().version`.
    pub current_version: String,
    /// When the last check completed, so the row can say "up to date" only
    /// after a real check rather than before the first one.
    pub last_checked_at: Option<String>,
}
```

`last_checked_at` is what separates "we have not looked yet" from "we looked
and there is nothing newer" — without it the Settings row would claim the app
is up to date before the first check has ever run. `chrono` is already a
workspace dependency, so the timestamp needs no new crate.

**Download progress.** The progress bar cannot ride on either of the two
channels above: the query is never refetched on its own
(`staleTime: Infinity`), and the event fires only on the transition *to*
`Ready`. So the status query carries a `refetchInterval` that is active only
while a download is actually running:

```ts
refetchInterval: (query) =>
  query.state.data?.status.kind === "downloading" ? 2000 : false
```

That polls only during the download — a few times over a few minutes — and
stops completely afterwards. It needs no third event channel and no
throttling logic in Rust.

### 7.8 Frontend layout

```
src/features/updater/index.ts
src/features/updater/data/update-status.ts    ← useUpdateStatus, useInstallUpdate, useCheckForUpdates
src/features/updater/data/update-prompt.ts     ← useUpdatePrompt: derived open state
src/features/updater/components/UpdateReadyDialog.tsx
src/features/updater/components/UpdateStatusRow.tsx
src/features/updater/components/index.ts
```

- `UpdateReadyDialog` is mounted in `AppLayout`
  (`features/app/components/AppLayout.tsx`), which is the shell for the
  main window's authenticated area. That single mount point satisfies two
  constraints at once:
  - **Not before login.** `AppLayout` sits under the `/_authed` guard, so a
    logged-out user is on `/login` and never reaches it. The update prompt
    never covers the login form or the bootstrap splash.
  - **Not in a workspace window.** `/_authed/project/$projectCode` mounts
    `ProjectWorkspaceLayout`, a different component. `AppLayout` is mounted
    by exactly one route — `/_authed/_layout` — so no window-label check is
    needed to keep the dialog out of workspace windows.

  It owns its own hooks, the same way `SettingsSyncBridge` owns
  `useHydrateSettingsFromStore` — it is not page-owned, so the "pages own the
  hooks" convention does not apply to it.
- `UpdateStatusRow` is rendered by `SettingsPage`. Following the repo's
  drawer/dialog convention, the page owns the hooks
  (`useUpdateStatus`, `useCheckForUpdates`, `useInstallUpdate`) and passes
  the values down as props; the component itself imports no API module.
- Because both consumers live in the authenticated shell, the status query
  only ever runs after login. The Rust worker still checks and downloads in
  the background regardless of login state — the "background" requirement is
  unaffected — the frontend simply has no UI to show until the user is in.
- Cross-feature imports go through the barrel: `SettingsPage` and
  `AppLayout` both import from `"../../updater"`. Unlike
  `features/app/index.ts` — which is empty because only `main.tsx` mounts its
  components — the updater barrel is non-empty, because two features consume
  it.
- Query keys are added to `src/shared/query/keys.ts` as
  `queryKeys.updater.status()`.

### 7.9 Deferral ("Later")

Two layers, for two different spans:

- **Within a session:** the dialog's open state is derived from
  `status == Ready && version !== deferredVersion`, where `deferredVersion`
  is component state. Clicking "Later" sets it and the dialog closes.
- **Across launches:** the deferred version string is written to
  `settings.bin`, alongside the existing `activeBranch` helpers. On the next
  launch the status query returns `Ready` again, the frontend compares
  versions, finds a match, and does not open the dialog.

The deferral lives in `features/settings/data/persist.ts` — as
`readUpdateDeferral` / `writeUpdateDeferral`, next to
`readActiveBranch` / `writeActiveBranch` — and is re-exported from the
`features/settings` barrel, which `features/updater` consumes. It cannot live
inside `features/updater`: `persist.ts`'s `getStore()` singleton is
module-private, so a second feature cannot reach the store without going
through the settings feature's own API. The deferral is desktop-local
persisted state about the app, exactly like the active branch, so the
settings feature is its natural home.

**The deferral must be loaded before the dialog can decide.** The status
query and the store read race: if the status resolves first, the dialog would
open and then close when the deferral lands — a visible flash of the update
modal on every launch, which defeats the whole point of deferring. So
`useUpdatePrompt` holds a `deferralLoaded` flag and only treats the update
as promptable once the store read has settled.

**The deferral is not cleared on install.** On Windows the process exits
inside `install`, so the frontend never gets the chance to write the store
afterwards, and clearing it before the call would lose the deferral if the
install failed. Leaving it is harmless: the deferral is keyed on a version
string, and every subsequent release announces a different one, so a stale
deferral can only ever suppress a *re-release of the same version number* —
which the artifact cache would have invalidated anyway.

### 7.10 Errors and logging

Both the worker and the commands log through `tracing`, matching the rest of
the desktop backend. A failed check sets the status to `Failed { message }`
and logs; it does not retry until the next hourly cycle, and it does not
surface a dialog — a failed background check is not an interruption. The
Settings row shows the failure inline and the manual "Check for updates"
button is the retry.

The updater has its own internal error type (`updater::UpdateError`) so the
plugin's error variants are not leaked into the transport layer. At the
command boundary it maps onto `http::dto::ApiError`, which the frontend
already knows how to narrow.

That mapping needs **one new `ApiError` variant**:

```rust
#[error("update failed: {message}")]
Update { message: String },
```

which serialises to `kind: "update"`. The existing variants do not fit: a
failed check or download is a transport failure (`Network` covers it), but a
signature-verification failure, a manifest that cannot be parsed, and an
installer that will not launch are none of `network` / `http` /
`refreshFailed` / `notImplemented` / `store` / `parse`, and forcing them
into `parse` would make the frontend's message misleading.

Per the guideline that wire-shape changes update both halves in the same
commit, this touches three files:

- `src-tauri/src/http/dto.rs` — the new variant.
- `src/shared/api/types.ts` — the TS mirror of `ApiError`.
- `src/shared/api/error.ts` — the `errorMessage` switch is exhaustive over
  `kind`, so it gains an `update` arm.

### 7.11 i18n

New keys in `lib/packages/ui/src/i18n/locales/en.ts` and `zhCN.ts` (both
catalogues must carry every key — `TranslationKey` is `keyof typeof en` and
`zhCN` is checked against it):

```
updater.title                 'Aegis {version} is ready to install'
updater.message               'The update has been downloaded and verified. Install it now, or do it later from Settings.'
updater.action.install        'Update now'
updater.action.later          'Later'
updater.action.check          'Check for updates'
updater.status.current        'Installed version: {version}'
updater.status.checking       'Checking for updates…'
updater.status.downloading    'Downloading update… {percent}%'
updater.status.ready          'Update available: {version}'
updater.status.upToDate       'Aegis {version} is up to date.'
updater.status.failed         'Update check failed: {message}'
updater.error.installFailed   'Failed to install the update: {message}'
```

### 7.12 The release server

Two things about the release server are outside the desktop app but
prerequisites for the flow working at all.

**(a) The manifest shape is wrong.** The manifest currently served at
`http://localhost:3000/aegis-desktop/releases/manifest.json` is:

```json
{
  "version": "0.1.0",
  "notes": "Aegis Release 0.1.0",
  "pub_data": "2026-11-18T06:05:10.294Z",
  "plaform": {
    "windows-x86_64": { "signature": "…", "url": "…" }
  }
}
```

Two defects, both confirmed by running the plugin's own `RemoteRelease`
deserializer against the live response:

1. `plaform` must be `platforms`. As served, the manifest fails to parse
   with `the 'url' field was not set on the updater response` — the
   misspelled key is ignored, `platforms` is absent, and the deserializer
   falls through to its dynamic branch looking for a top-level `url`.
   `pub_data` must likewise be `pub_date`; that one is benign (the field is
   simply absent), but it loses the release date.
2. `version` is `0.1.0`, identical to the running app's version. The
   plugin's default comparator offers an update only when
   `release.version > current_version`, so even a correctly-shaped manifest
   would report "no update". Exercising the download path requires the
   manifest to advertise a version above the app's.

The release server is outside this repository. Either the manifest generator
is fixed, or its path is given so it can be fixed here. Either way the
desktop side does not change: it consumes the plugin's documented manifest
shape.

**The test release must be a genuine 0.1.1 build, not the 0.1.0 artifact
relabelled.** The signature covers the artifact bytes, so pointing a
`"version": "0.1.1"` manifest at the existing signed 0.1.0 installer would
pass verification — but installing it would put **0.1.0** back on the
machine, and the next launch would see 0.1.1 > 0.1.0 again and offer the
same update forever. The release therefore has to be rebuilt with version
0.1.1 and signed with the same key. The local server already holds that
signing setup, since it produced the signed 0.1.0 artifact.

**(b) The signing key is correct, but the signatures carry no version.** The
pubkey in `tauri.conf.json` decodes to minisign key id `D7C402AB6CCF5CE1`,
and the signature in the served manifest was produced by the same key — the
key-id bytes and key material match. No re-keying is needed.

What the signatures *do* lack is a version. The artifact signature's trusted
comment is:

```
timestamp:1794972726	file:aegis-desktop_0.1.0_x64-setup.exe
```

The plugin's `signed_version` looks for a tab-separated `version:` field,
finds none, and therefore treats the signature as carrying no version. That
is harmless for the normal path — `Config::require_signed_version` defaults
to `false`, so `verify_signature` accepts it — but it is why the design does
**not** use `Updater::restore_update`: that method forces
`require_signed_version = true`, which would reject these signatures with
`MissingSignedVersion`.

Re-signing the release with the Tauri CLI's `--app-version` flag would add
the `version:` field and let the plugin enforce the binding between the
announced version and the signed artifact. That is a hardening step for the
release server, not a blocker for this design, and it is not required by it.

**(c) The endpoint is `http://localhost:3000/...`** with
`dangerousInsecureTransportProtocol: true` — a development-only
configuration that has to be repointed at the real release server before any
of this ships.

## 8. Testing

### Rust (`#[cfg(test)] mod tests`, no network)

- `status.rs` — each transition, and that an unchanged or older version does
  not leave `Ready`.
- `service.rs` — against `FakeUpdateSource`: check finds nothing → `Idle`;
  check finds a release → `Downloading` → `Ready`; a second cycle for the
  same version does not download again and does not emit again; an *older*
  announced version does not replace the pending one; a download error →
  `Failed`; a verification failure → `Failed`, not `Ready`; a cached
  artifact verifies and skips the download; a cached artifact that fails
  verification is discarded and re-downloaded; `install_ready` calls
  `PendingRelease::install` with the downloaded bytes and errors when there
  is nothing ready; a second check while one is in flight is a no-op.
- `source.rs` — `PluginRelease` maps the plugin's `Update` to the exposed
  version / notes / date.
- A **manifest-shape contract test** that pins the plugin's expected
  manifest shape: a manifest with `platforms` parses; the `plaform`
  misspelling does not. This is the regression test for the defect in
  §7.12(a).

### TypeScript (vitest)

- `useUpdatePrompt` — opens when the status is `Ready`; stays closed while
  the deferral is still loading; stays closed when the version matches the
  persisted deferral; stays closed after a session-scoped deferral.
- `UpdateReadyDialog` — renders the version; "Update now" calls the install
  mutation; "Later" closes it and persists the deferral; the install error
  renders in an `Alert`.
- `AppLayout` — mounts the dialog, and the dialog is not reachable from the
  login or bootstrap routes (that is a routing property of `/_authed`, so it
  is covered by the existing route tests rather than a component test).
- `UpdateStatusRow` — shows "up to date" only after a check has completed;
  shows "update available" with the version; shows download progress; and
  the "Check for updates" button triggers a check.
- The event listener invalidates the status query.

## 9. Verification gate

```bash
# Frontend
pnpm --filter aegis-desktop typecheck
pnpm --filter aegis-desktop test
pnpm --filter aegis-desktop build

# Backend
cargo fmt --all -- --check
cargo clippy -p aegis-desktop --all-targets --all-features -- -D warnings
cargo test -p aegis-desktop
```

End-to-end, on a machine with the release server running: publish a release
whose version is above `0.1.0`, launch the app, watch the check fire and the
artifact download, confirm the dialog appears, defer it, confirm it does not
reappear on the next hourly cycle or the next launch, and confirm the second
launch does not re-download the artifact. Then install from Settings and
confirm the app restarts on the new version.

Note on this environment: the Rust toolchain has been observed to OOM while
compiling the `windows` crate and `cargo_metadata` on this machine (16 GB
total, ~3 GB free at the time). `CARGO_BUILD_JOBS=1` was enough to get the
build through. If a build dies with `rustc-LLVM ERROR: out of memory`, retry
with `CARGO_BUILD_JOBS=1` before suspecting the code.

## 10. What the builder still needs to decide

- **Exact module/function names** for the pieces the design describes by
  intent rather than by name (`UpdateConfig` fields beyond `interval`,
  the internal name of the shared-status mutex wrapper).

One prerequisite sits outside this repository and is **not** the builder's
to close: the release manifest fix in §7.12(a), and publishing a genuine
0.1.1 build signed with the existing key, so the flow can be exercised end
to end. The desktop side is built against the plugin's documented manifest
shape either way.

## 11. Decisions taken during design

These were settled in conversation; they are recorded here so the builder
does not reopen them.

| Decision | Rationale |
|---|---|
| The background loop lives in Rust, not TypeScript. | `main.tsx` runs in every window; one loop per process is correct by construction. |
| The Rust service owns the artifact; the frontend only renders. | Makes "download first, then ask" a guarantee, and keeps install instant. |
| The dialog appears in the main window only. | Workspace windows have their own layout; an update dialog there would be confusing. |
| The dialog appears only after login, and only in the main window. | A modal update prompt over the login form would be an interruption the user cannot dismiss usefully. Mounting it in `AppLayout` — which only `/_authed/_layout` mounts — satisfies both constraints at once. |
| The Settings row is the only place a pending update is visible. | A quiet indicator, not a nag; no sidebar badge, so `AppLayout`'s sidebar and the shared `Sidebar` in `@aegis/ui` stay untouched. |
| The dialog opens at most once per version per session. | The hourly worker must not turn into hourly nagging. |
| Deferral persists across launches for the same version. | The point of deferring is to not be asked again; the Settings row keeps it discoverable. |
| A "Check for updates" button is added to the Settings row. | The quiet indicator needs a manual trigger and a way to retry a failed check. |
| Download progress is shown in the Settings row, not the dialog. | The dialog only opens once the artifact is ready, so there is nothing to show there. |
| The updater reuses `ApiError` as its command error type, with one new `update` variant. | Keeps one error type per boundary and lets the frontend reuse `errorMessage`; the existing variants do not describe updater failures. |
| The updater is a new module, not a `http/` module. | It talks to the release server, not `aegis-server`; the transport-only rule is about `aegis-server`. |
| No shutdown handshake for the worker. | A partial download has nothing to drain; the next launch re-checks. |
| The artifact is cached on disk under `app_data_dir`. | Otherwise a deferred update is re-downloaded on every launch, forever. |
| The `Update` object is not cached; only the bytes are. | `restore_update` forces `require_signed_version`, which the release server's current signatures do not satisfy. |
| `Ready` means downloaded *and* verified. | The plugin does not verify on download; without an explicit `verify` call a tampered artifact would only be caught at install time. |
| The deferral is stored by the settings feature, not the updater feature. | `persist.ts`'s store singleton is module-private; the deferral is desktop-local state about the app, like the active branch. |

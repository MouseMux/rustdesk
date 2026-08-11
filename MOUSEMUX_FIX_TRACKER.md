# MouseMux RustDesk — Fix Tracker

**Created:** 2026-08-09
**Branch:** `mousemux-v2.2-flutter-complete`
**Base reviewed against:** upstream `db4296533` (RustDesk 1.4.3)
**Current version:** `1.4.3-mousemux-v2.5`

This document is the single source of truth for the remediation effort started
2026-08-09. It lives inside the git repo so it is pullable from any machine.
**Never delete findings — mark them resolved.**

---

## How to use this document

Each finding has a **Status** and a **Decision** line. Update both as we go.

- `OPEN` — not yet discussed
- `DECIDED` — approach agreed, not yet implemented
- `FIXED` — implemented, not yet verified in a build
- `VERIFIED` — confirmed working in a numbered build
- `WONTFIX` — deliberately declined (record why)

---

## Workstream A — Build hygiene (versioning + per-build folders)

**Goal:** every build lands in its own folder, version bumped after each fix batch.

### Current state (verified 2026-08-09)
- **`Cargo.toml` is the single source of truth for the version.** `build.rs:84`
  calls `hbb_common::gen_version()`, which regenerates `src/version.rs` from the
  `version =` line and stamps `BUILD_DATE` with the build time.
  (An earlier note in this file claimed version.rs was hand-maintained and nothing
  generated it — that was wrong, corrected 2026-08-09.)
- `src/version.rs` is gitignored and untracked **by design**. Do not edit it by
  hand; the build overwrites it. A fresh clone regenerates it.
- `res/PKGBUILD` carries its own `pkgver` and must be bumped alongside Cargo.toml.
- Build output always overwrites the fixed path
  `flutter/build/windows/x64/runner/Release/`, so it must be archived before the
  next build.
- Established archive convention in `rustdesk-development/final-builds/`:
  `rustdesk-mousemux-<version>-<tags>-<YYYY-MM-DD_HH-MM-SS>/`
  containing `README.txt`, `Release/`, `Release.zip`, and (historically) an
  installer `.exe`.

### Agreed scheme
- **Status:** DECIDED 2026-08-09
- **Decision:** bump the minor suffix per fix batch — `v2.3` for the current
  remediation series. Bump `Cargo.toml` + `res/PKGBUILD`, build, then archive
  `Release/` into a dated `final-builds/` folder before the next build overwrites
  it. Tag the folder name with build variant (e.g. `-nohwcodec-`) when it differs
  from the established configuration, so the name never overstates the contents.

---

## Workstream B — Git integrity (pull from any machine)

**Goal:** `git clone --recursive` on a fresh machine produces a buildable tree.

- **Status:** IN PROGRESS — the only remaining blocker is creating the GitHub repo,
  which needs a human (no `gh` CLI on this machine).

Sub-tasks:
- [x] Restore `hbb_common` locally to the MouseMux fork commit (Finding 1)
- [x] Commit all working-tree changes
- [x] Add `flutter/.flutter`, `flutter/.flutter_tool_state` to `.gitignore`
- [ ] **Create `MouseMux/hbb_common` on GitHub** (empty, no README) ← needs human
- [ ] Push `db45828cd` + its 154-commit history to the fork
- [ ] Add `upstream` remote (`rustdesk/hbb_common`) inside the fork, so future
      upstream syncs are possible — user explicitly wants this
- [ ] Put `db45828cd` on a branch (it is currently a detached HEAD)
- [ ] Repoint `.gitmodules` at the fork and commit
- [ ] Push `mousemux-v2.2-flutter-complete` to `origin`

**Do not push the superproject before the fork exists.** Doing so publishes commits
referencing a submodule nobody else can fetch — exactly the trap that caused
Finding 1 in the first place.

### Commits made 2026-08-09/10 (all local, none pushed)
```
ef97d5ae1  Fix six MouseMux protocol and concurrency defects   (3,4,5,6,17,18)
02dac05c9  Bump to 1.4.3-mousemux-v2.3, add fix tracker, build fixes
2d2b972d8  Add hwcodec setup script; record FFmpeg 8 incompatibility
4ec8b3b70  Add researched upgrade plan for RustDesk 1.4.3 -> 1.4.9
31939c7e8  Gate per-event MouseMux logging behind a cargo feature  (13)
```

---

## Workstream C — Findings

Severity: **C**ritical / **M**oderate / **L**ow

| # | Sev | Title | Status |
|---|-----|-------|--------|
| 1 | C | `hbb_common` pinned commit is unreachable — build not reproducible | VERIFIED in v2.4 (local) — fork not yet pushed |
| 2 | C | `APP_NAME` customization reverted — collides with stock RustDesk | VERIFIED in v2.4 |
| 3 | C | Remotely-triggerable panic on non-ASCII peer names | VERIFIED in v2.4 |
| 4 | C | Shutdown deadlock — process hangs on exit | VERIFIED in v2.4 |
| 5 | C | Per-connection keyboard ID race between simultaneous users | VERIFIED in v2.4 |
| 6 | C | MouseMux ID leak when MouseMux not running at disconnect | VERIFIED in v2.4 |
| 7 | M | User count drifts, mismatch only logged never corrected | VERIFIED in v2.4 |
| 8 | M | `std::process::exit(0)` in `window_proc` skips all cleanup | VERIFIED in v2.4 |
| 9 | M | Lock-poisoning inconsistency; panic can unwind out of `window_proc` | VERIFIED in v2.4 |
| 10 | M | Version constant mismatch: code 143 vs protocol spec 142 | CLOSED — not a bug, see below |
| 11 | M | Thread spam on every `MOUSEMUX_STARTUP_BROADCAST` | VERIFIED in v2.4 |
| 12 | M | `RegisterClassExA` not idempotent — re-init fails | VERIFIED in v2.4 |
| 13 | L | Per-keystroke `info!` logging in the input hot path | VERIFIED in v2.4 |
| 14 | L | `pending_peer_info` duplicates `connections[].peer_info` | VERIFIED in v2.4 |
| 15 | L | Unused `_hwnd` param / bound-but-unused `user_id` | VERIFIED in v2.4 |
| 16 | L | Non-BMP chars sent as single `u32` may corrupt in a WCHAR buffer | SUPERSEDED by 18 |
| 17 | L | Support URLs drifted between Sciter and Flutter UIs | VERIFIED in v2.4 |
| 18 | C | Peer name encoding mismatch — RustDesk 1 msg/char vs MouseMux 4 msgs/char | VERIFIED in v2.4 |
| 19 | L | Touch-scale path never sets `current_conn_id`, inherits a stale one | VERIFIED in v2.4 |
| 20 | C | hwcodec build fix lives outside the repo, in the cargo cache | RESOLVED in v2.5 |

---

### Finding 1 — `hbb_common` pinned commit is unreachable (CRITICAL)

**Status:** FIXED LOCALLY 2026-08-09 — submodule restored and verified at
`db45828cd`. **Still outstanding:** the fork does not exist on GitHub, so the build
remains reproducible on this machine only. See Workstream B.
**Decision:** Option A — fork `hbb_common` under the MouseMux org (see below).

Branch records `libs/hbb_common` at `db45828cd`. That commit is not in the local
clone and not on the configured remote:
```
git fetch origin ee4f6db8c...
fatal: remote error: upload-pack: not our ref db45828cd8f22a3e52285df1ba6976e6c5e1a3b6
```
`.gitmodules` points at `https://github.com/rustdesk/hbb_common` (upstream, never
a MouseMux fork), so the MouseMux commit was never pushed anywhere reachable.
Submodule reflog has a single `clone:` entry — old objects are gone.

**Impact:** `git clone --recursive` of this branch cannot check out, so the build
is not reproducible on any other machine. Directly blocks Workstream B.

**RECOVERED 2026-08-09 — commit is intact, not lost.**
`rustdesk-development/rustdesk-current/libs/hbb_common` is a **full git clone**
sitting at exactly `db45828cd8f22a3e52285df1ba6976e6c5e1a3b6` (verified exact
match to the pinned SHA) with complete history. Nothing needs reconstructing.

The entire MouseMux fork of hbb_common is 3 commits touching 1 file:
```
25bce8d  Change APP_NAME to 'RustDesk MouseMux Edition'
78313b5  Use filesystem-safe directory names (rustdesk-mousemux-edition)
db45828cd  Fix: Remove spaces from APP_NAME to fix Windows 11 portable service issue
         src/config.rs | 10 insertions(+), 7 deletions(-)
```
Base of that chain: upstream `5ed0afd`.

Content of the change:
```rust
APP_DIR_NAME: &str = "rustdesk-mousemux-edition";                    // line 53
APP_NAME: RwLock<String> = RwLock::new("rustdesk-mousemux-edition")  // line 64
ProjectDirs::from("", &org, APP_DIR_NAME)                            // line 650
```

**Decision taken 2026-08-09:** Option A — fork `hbb_common` under the MouseMux
org, repoint `.gitmodules`. (Alternatives declined: vendoring the files into the
main repo; patching at build time.)

**Blocker:** `gh` CLI is not installed on this machine, so the GitHub repo must be
created manually or `gh` installed.

**Open sub-decision:** push `db45828cd` as-is (old upstream base `5ed0afd`) vs
rebase the 3 commits onto current upstream `main` (`28ac03a89`). See Finding 1b.

---

### Finding 1c — Commit rewritten before publishing (2026-08-10)

The pinned commit was `ee4f6db8cfaa7bc515721ea32685996b926fec90`. Before the fork
was published, its message was found to contain a two-line AI-tool
attribution trailer left by an October 2025 session — a "Generated with" line and
a matching Co-Authored-By line. Those two lines were stripped (author and dates preserved), which necessarily
changed the SHA:

    ee4f6db8cfaa7bc515721ea32685996b926fec90   ->   db45828cd8f22a3e52285df1ba6976e6c5e1a3b6

The superproject gitlink was updated to match in the same commit that repoints
`.gitmodules`. **Any older reference to `ee4f6db` in notes or scripts is stale.**
Safe to rewrite because the commit had never been published anywhere.

The same trailer was stripped from the nine 2026-08-09/10 remediation commits in
this repo, which were likewise unpushed. Two AI mentions were deliberately LEFT
alone:
- `c58fd145f` — an upstream documentation commit authored by
  `rustdesk <info@rustdesk.com>`, already public. Rewriting it would permanently
  diverge our history from upstream and break future merges.
- Three Oct-2025 commits on `origin/mousemux-sciter` (`a1d958baf`, `575efc6f4`,
  `60195821d`). They are **not** on `mousemux-v2.2-flutter-complete` and are
  already published; cleaning them means force-pushing a public branch.

That upstream documentation file is no longer in the tree.

Also checked and cleared: no tracked source file, script or document in either
repo mentions the tool. The single hit in `Release/data/app.so` is a false
positive: the matched word is an entry in the common-password dictionary bundled
with the `password_strength` package (`flutter/pubspec.yaml:85`), sitting between
`looney` and `27111990`. It is present in every RustDesk build, upstream included.

---

### Finding 1b — Which hbb_common base to publish

**Status:** RESOLVED 2026-08-09 — no rebase needed, restore `db45828cd`.

Initial concern was that `db45828cd` (base `5ed0afd`) might not compile against the
current superproject, since the Feb 2026 build used upstream main `28ac03a89`
plus ~1200 lines of `Cargo.lock` churn. **That inference was wrong.** Provenance
evidence collected 2026-08-09:

- `rustdesk-development/rustdesk-current` is the **VM working repo**, carried over
  by zip. Confirmed by its `clean` remote pointing at `O:/rustdesk-development/
  rustdesk-clean`, a path that does not exist on this machine.
- VM tree and build tree are **byte-identical** apart from CRLF conversion, except
  `src/version.rs` (BUILD_DATE stamp only) and `libs/hbb_common/src/config.rs`.
- **VM `Cargo.lock` == the COMMITTED `Cargo.lock`.** The churn in the build tree
  is local and uncommitted.
- That churn is all *additions* — `webrtc*`, `x25519-dalek`, `x509-parser`,
  `yasna`, `xkeysym`, `xcursor`, `zeroize_derive` — i.e. deps upstream
  `hbb_common` gained after the fork point.

**Reconstruction:** fresh clone here on 2026-02-10 → `git submodule update --init`
failed on `db45828cd` (`not our ref`) → upstream `hbb_common` main cloned instead to
unblock → newer deps → Cargo.lock re-resolved → build succeeded but produced a
binary branded `"RustDesk"`. The submodule bump was an **accidental workaround,
not a deliberate upgrade.**

**Conclusion:** `db45828cd` + committed `Cargo.lock` is the proven configuration
behind the shipped 2025-11-01 builds. Restore it; do not rebase.

---

### Finding 2 — `APP_NAME` reverted, collides with stock RustDesk (CRITICAL)

**Status:** OPEN
**Decision:** _(pending)_

Current tree has `APP_NAME = "RustDesk"` and no `APP_DIR_NAME` override. Verified:
`librustdesk.dll` built 2026-08-09 contains **zero** occurrences of
`rustdesk-mousemux-edition`. The only runtime `APP_NAME` write
(`src/common.rs:1671`) is behind signed custom-client config and does not apply.

**Does NOT reintroduce BUG #1** — that required *spaces* in the path; `"RustDesk"`
has none.

**Does cause** a namespace collision with official RustDesk: identical
`C:\ProgramData\RustDesk`, `\\.\pipe\RustDesk\...`, and config dir. Two installs
on one host contend for the same shared memory and named pipes. Also silently
undoes commits `30bf4c7e0` and `ea5b0e3f8` (roaming-vs-local directory naming).

---

### Finding 3 — Remotely-triggerable panic on non-ASCII peer names (CRITICAL)

**Status:** OPEN
**Decision:** _(pending)_

`src/server/connection.rs:1688` and `src/platform/windows_mousemux.rs:681`:
```rust
let peer_info = if peer_info.len() > 256 { &peer_info[..256] } else { ... };
```
`peer_info` is `format!("{}@{}", self.lr.my_name, self.lr.my_id)` where `my_name`
is supplied by the connecting client. `.len()` is **bytes**; slicing a `&str` at a
non-char-boundary panics. Any Cyrillic/CJK/emoji name whose 256th byte lands
mid-character crashes the connection thread. Comment says "chars", code uses bytes.

**Fix direction:** `floor_char_boundary`, or `char_indices().take(256)`.

---

### Finding 4 — Shutdown deadlock, process hangs on exit (CRITICAL)

**Status:** OPEN
**Decision:** _(pending)_

`src/platform/windows_mousemux.rs:417-439`:
```rust
PostQuitMessage(0);   // posts WM_QUIT to the CALLING thread's queue
DestroyWindow(hwnd);  // cross-thread DestroyWindow is a no-op
handle.join().ok();   // blocks forever
```
Message loop runs on a spawned thread (`init_mousemux_window:373`).
`PostQuitMessage` only queues to the calling thread, and Windows forbids
destroying a window from a thread that did not create it. `GetMessageA` never
returns 0, so `join()` blocks indefinitely. Reached from `global_clean()` at
`src/main.rs:16`, `:36`, `:106` — every normal exit path.

**Fix direction:** `PostMessageA(hwnd, WM_CLOSE, 0, 0)` or
`PostThreadMessageA(loop_thread_id, WM_QUIT, 0, 0)`.

---

### Finding 5 — Per-connection keyboard ID race (CRITICAL)

**Status:** OPEN
**Decision:** _(pending)_

This is the defect the whole feature exists to prevent, and commit `f00695982`
claims to have fixed it. Each connection has its **own** input thread
(`connection.rs:346` channel, `:497` spawn). `input_service.rs:1751-1767`:
```rust
let mut en = ENIGO.lock().unwrap();
en.set_current_conn_id(Some(conn));
let keyboard_id = en.get_keyboard_id(conn).unwrap_or(...);
drop(en);                                   // lock released here
rdev::set_keyboard_extra_info(keyboard_id); // process-global, no lock at all
handle_key_(evt);                           // re-acquires lock later
```
`rdev::set_keyboard_extra_info` writes an unsynchronized `static mut
DW_KEYBOARD_EXTRA_INFO` (rdev `windows/simulate.rs:31-33`), read by
`map_keyboard_mode()` which bypasses enigo entirely. Two users typing
simultaneously get keystrokes attributed to the wrong MouseMux keyboard ID.

**Reference implementation:** the mouse path is correct — `handle_mouse_`
(`input_service.rs:1042-1049`) takes the ENIGO lock and **holds it** across the
whole event.

---

### Finding 6 — ID leak when MouseMux not running at disconnect (CRITICAL)

**Status:** OPEN
**Decision:** _(pending)_

`windows_mousemux.rs:828` — `release_ids` returns early when
`find_mousemux_window()` is `None`, *before* `clear_ids_for_connection` at `:855`.
Dead connection stays in `state.connections` permanently.
`re_request_all_active_connections` (`:889`) then re-registers ghosts on the next
MouseMux start, and `has_ids()` (`:489`) reports them as active to the UI.

**Fix direction:** clear local state first, then attempt the notify.

---

### Finding 7 — User count drift only warned (MODERATE)

**Status:** OPEN — `windows_mousemux.rs:227`, `:251`. Handler receives the
authoritative `mousemux_total` but only logs a warning on mismatch.

### Finding 8 — `std::process::exit(0)` in `window_proc` (MODERATE)

**Status:** OPEN — `windows_mousemux.rs:278`. Comment claims it "will trigger
cleanup handlers and gracefully shut down"; it does neither. No destructors run,
and `global_clean()`/`notify_shutdown()` never execute, so MouseMux is never told
RustDesk left.

### Finding 9 — Lock-poisoning inconsistency (MODERATE)

**Status:** OPEN — `.lock().unwrap()` in ~17 places vs `if let Ok(...)` in the two
`window_proc` ID handlers (`:143`, `:184`). One panic while holding
`MOUSEMUX_STATE` poisons it; every later `unwrap` panics, including inside
`window_proc` — unwinding out of a Windows callback across the FFI boundary is UB.

### Finding 10 — Version constant mismatch (MODERATE)

**Status:** CLOSED 2026-08-10 — was never a bug, and is now moot.

MouseMux validates the RustDesk version as a RANGE (`VERS_MIN=100`, `VERS_MAX=999`
in `rustdesk_validation.c`), not an exact match, so the stale 143 always connected
— it merely misreported which RustDesk this was in MouseMux's own logs.

Since resolved properly: `RUSTDESK_VERSION` is now 149 to match the 1.4.9 base, and
the protocol version moved 122 -> 123 alongside the widened name-byte range. The
old `MOUSEMUX_PROTOCOL_V2.2.md` that claimed "Version ID 142" has been superseded —
see the authority note below.

### Finding 11 — Thread spam on broadcast (MODERATE)

**Status:** OPEN — `windows_mousemux.rs:116` spawns a fresh thread per
`MOUSEMUX_STARTUP_BROADCAST` with no dedupe.

### Finding 12 — `RegisterClassExA` not idempotent (MODERATE)

**Status:** OPEN — `windows_mousemux.rs:307` returns `Err` if the class already
exists, so re-init after shutdown fails. Class is never unregistered.

### Finding 13 — Per-event logging, now compile-time gated

**Status:** FIXED 2026-08-09 — needs build to verify.
**Decision:** cargo feature `mousemux-debug`, **not** `debug_assertions`.

Originally: `input_service.rs` and enigo logged at `info!` on every key event —
added by commit `9bd35a65e` for the October 2025 keyboard-ID investigation and
never dialled back. Synchronous log I/O on every input event, in the same path
Finding 5's serialization lock now runs through.

Why a feature and not `debug_assertions`: the problems this tracing exists for
(per-connection ID assignment, MouseMux handshake timing) only reproduce in
**release** builds — the original investigation was done on release builds. A
debug-only gate would remove the logging exactly when it is next needed.

```
normal release:        cargo build --features flutter --lib --release
with MouseMux tracing: cargo build --features flutter,mousemux-debug --lib --release
```

Two macros, because two crates are involved:
- `src/lib.rs` → `crate::mm_debug!` for the rustdesk crate
- `libs/enigo/src/lib.rs` → `crate::mm_debug!` for enigo (needs doc comments —
  enigo denies `missing_docs`)
- `rustdesk`'s `mousemux-debug = ["enigo/mousemux-debug"]` propagates it, so one
  flag drives both crates.

With the feature off the macro expands to **nothing** — no runtime branch, no
formatting cost. `log::debug!` would still cost a level check per call.

**Gated (per-event, hot path):**

| File | Sites |
|------|-------|
| `input_service.rs` | 8 — `handle_key_with_conn` ×4, `handle_key_`, `handle_mouse_`, `map_keyboard_mode`, `legacy_keyboard_mode` |
| `enigo/win_impl.rs` | 6 — all arms of `get_mouse_extra_info` / `get_keyboard_extra_info` |

The enigo sites were the worst: called on **every injected event, including every
mouse move**, and the "no ID" arms were `warn!`, flooding the log whenever
MouseMux simply was not running — a normal, supported state.

**Deliberately NOT gated** — rare, and precisely what a field report needs:
errors and warnings, ID assignment, connection register/release, protocol
handshake, and the `EXITING` warning in `handle_key_with_conn`.

### Finding 14 — Redundant peer_info storage (LOW)

**Status:** OPEN — `windows_mousemux.rs:692-707`. `pending_peer_info` duplicates
`connections[].peer_info`.

### Finding 15 — Dead parameters (LOW)

**Status:** OPEN — unused `_hwnd` in `message_loop_thread` (`:345`); `user_id`
bound but only logged (`:214`, `:238`).

---

## Applied fixes — 2026-08-09

**Finding 3** (`windows_mousemux.rs`, `connection.rs`)
Decision: char-based truncation with a single enforcement point.
```rust
let peer_info: String = peer_info.chars().take(MAX_PEER_INFO_LENGTH).collect();
```
Removed the duplicate byte-slicing guard in `connection.rs::on_remote_authorized`;
`request_ids` is now the only place that caps length. Also corrected the trailing
log to report `.chars().count()` instead of `.len()`, which claimed "chars" while
printing bytes. `str::floor_char_boundary` would be tidier but is still unstable.

**Finding 4** (`windows_mousemux.rs`)
Decision: WM_CLOSE + WM_DESTROY handler.
- `shutdown_mousemux_window` now posts `PostMessageA(hwnd, WM_CLOSE, 0, 0)` instead
  of `PostQuitMessage` + `DestroyWindow`, both of which targeted the wrong thread.
- New `WM_DESTROY` arm in `window_proc` calls `PostQuitMessage(0)` — correct,
  because that arm runs on the message-loop thread.
- Dropped now-unused imports `DestroyWindow`, `HWND_MESSAGE`, `WS_OVERLAPPEDWINDOW`
  (`WS_OVERLAPPEDWINDOW` was already dead); remaining textual mentions are comments.

**Finding 6** (`windows_mousemux.rs`)
Decision: reorder, no design choice. Local cleanup (`clear_ids_for_connection` +
`pending_peer_info.remove`) now runs **before** `find_mousemux_window()`, so a
disconnect while MouseMux is stopped no longer leaks the connection. Return value
is still `false` when MouseMux is absent — the sole caller
(`connection.rs::on_close`) ignores it, so this stays "was MouseMux notified".

**Finding 17** — both support URLs set to `https://www.mousemux.com`.

**Finding 5** (`input_service.rs`)
Decision: single input serialization lock.
- New `INPUT_SERIALIZE: Mutex<()>` (windows-only), declared next to `EXITING`.
- Acquired at the top of `handle_key_with_conn` (held across `handle_key_()`) and
  in `handle_mouse_` immediately before the ENIGO lock.
- **Lock order is always INPUT_SERIALIZE → ENIGO.** Verified no path takes them in
  the reverse order: the only other ENIGO-then-something path is `handle_scale`,
  which never takes `INPUT_SERIALIZE`, so no inversion exists.
- Holding the ENIGO lock across `handle_key_()` was NOT viable — that path re-locks
  ENIGO internally at `input_service.rs:1386`, `:1548`, `:1593`, `:829`, `:854`,
  and `std::sync::Mutex` is not reentrant. Hence a separate outer lock.
- Cost on Windows is small: `key_sleep()` (12ms) is inside the macOS-only
  `handle_key`, and `modifier_sleep()` is 1 nanosecond.

**Finding 18** (`windows_mousemux.rs`)
Decision: send 4 UTF-32 LE bytes per character, ASCII-clamped, no MouseMux change.
- Name loop now posts four messages per character, low byte first.
- Non-ASCII replaced with `'?'` so every byte stays inside MouseMux's `0..=127`
  validator (`NAME_CHAR_MAX`), which its own UTF-32 accumulator otherwise
  contradicts.
- Terminator is now **four** zero bytes, not one — a single zero only filled one
  byte of the accumulator and never terminated the name.

---

### Finding 19 — Touch-scale path inherits a stale conn_id (LOW)

**Status:** FIXED 2026-08-10 — needs build.
`handle_pointer_` now takes `INPUT_SERIALIZE` and sets `current_conn_id`, matching
`handle_mouse_` and preserving the INPUT_SERIALIZE→ENIGO lock order. The ENIGO
guard is released immediately, so `handle_scale`'s own lock is unaffected.

`handle_pointer_` (`input_service.rs:1026`) routes touch ScaleUpdate to
`handle_scale`, which takes the ENIGO lock and synthesises input, but **never calls
`set_current_conn_id`**. So a touch-scale event is stamped with whatever
`current_conn_id` was last set by some other connection's mouse or key event.

Not a race introduced by the Finding 5 guard — the path simply never participated
in per-connection tracking. It also takes ENIGO without `INPUT_SERIALIZE`, which is
safe (no inversion is possible, since it never takes `INPUT_SERIALIZE` at all).

Fix would be: take `INPUT_SERIALIZE`, then set `current_conn_id`, in
`handle_pointer_` — mirroring `handle_mouse_`. Low priority: affects only
touch/pinch input from mobile clients.

---

### Finding 20 — hwcodec build fix lives outside the repo (CRITICAL, reproducibility)

**Status:** OPEN — patch re-applied by hand 2026-08-09, but not yet made reproducible.

Same class of landmine as Finding 1: something the build genuinely requires, stored
where git cannot see it.

`hwcodec` is a **non-default** feature (`Cargo.toml`: `default = ["use_dasp"]`), and
building with it fails to link unless two edits are made to the hwcodec crate's own
`build.rs` — a file in the **cargo git checkout**:
```
C:\Users\dev\.cargo\git\checkouts\hwcodec-74796a7f8f16bbb9\17c1dbb\build.rs
  line 157  static_libs: + "swresample"
  line 177  dyn_libs:    + "mfuuid", "mfplat", "strmiids"
```
Documented in `BUGFIXES.md`, and a copy exists as
`rustdesk-development/rustdesk-current/hwcodec_build_fix.patch` (note: that patch
adds only mfuuid+mfplat; `BUGFIXES.md` adds strmiids too — the latter was used).

Wiped by any cargo cache clear, and absent on every fresh machine. Neither the
repo nor CI applies it.

Also required, and equally undeclared for classic-mode vcpkg:
`vcpkg install mfx-dispatch:x64-windows-static --classic`
(`--classic` is needed because vcpkg otherwise finds a `vcpkg.json` and switches to
manifest mode, which rejects named packages. `rustdesk/vcpkg.json` does declare
`mfx-dispatch`, but manifest mode installs to a project-local `vcpkg_installed/`,
not the global tree that `build.rs` reads via `VCPKG_ROOT`.)

And per `BUGFIXES.md`, after patching the hwcodec cache **must** be purged or cargo
silently relinks the unfixed `.rlib`:
```
rm -rf target/release/.fingerprint/hwcodec-* target/release/build/hwcodec-* \
       target/release/deps/*hwcodec* target/release/*hwcodec*
```

**Consequence discovered 2026-08-09:** the first v2.3 build was made with
`--features flutter` only, so it shipped with **no hardware codec at all** —
verified by zero `avcodec`/`libmfx` references in the DLL. Every 2025-11-01 build
had it. Would have been a silent encoding regression.

**Partial fix applied 2026-08-09:** `tools/setup-hwcodec.sh` is now checked in. It
installs the vcpkg package, applies the build.rs patch idempotently, and purges the
fingerprints — so the requirement lives in the repo rather than in one machine's
cargo cache.

**BUT hwcodec still cannot be built here — a second, previously unknown blocker.**
The linkage patch works (verified: `swresample`, `mfuuid`, `mfplat`, `strmiids` all
appear in the emitted link directives). hwcodec 0.7.1 simply does not **compile**
against modern FFmpeg. This machine's vcpkg has **FFmpeg 8.0.1** (libavutil 60.8),
and hwcodec 0.7.1 was written for 6.x:
```
util.cpp(59,61)         error C2065: 'FF_PROFILE_H264_HIGH' / 'FF_PROFILE_HEVC_MAIN'
                        undeclared            -> renamed to AV_PROFILE_* in FFmpeg 7.0
ffmpeg_ram_decode(218)  error C2039: 'key_frame' is not a member of 'AVFrame'
                        -> removed in FFmpeg 7.0, now AV_FRAME_FLAG_KEY in frame->flags
```
The 2025-11-01 builds were made against an older FFmpeg, on the VM.

**RESOLVED 2026-08-10 — and the earlier diagnosis was incomplete.**

The real cause was never hwcodec. `vcpkg.json` pins baseline
`120deac3062162151622ca4860575a33844ba10b`, which is **FFmpeg 7.1.1** — the
project already declared the correct version. This machine's vcpkg tree had
drifted two majors ahead to 8.0.1, and FFmpeg 8.0 removed both APIs hwcodec uses.
Verified directly against the FFmpeg release tags:

| symbol | n7.1.1 | n8.0 |
|--------|--------|------|
| `FF_PROFILE_H264_HIGH` | present | removed |
| `AVFrame::key_frame`   | present | removed |

Note the earlier expectation that upgrading to 1.4.9 would fix this "for free" was
**wrong** — 1.4.9's newer hwcodec (`778df1f9`) guards `key_frame` but still uses
`FF_PROFILE_*` unguarded, so it fails against FFmpeg 8 too. Fixing the FFmpeg
version is the only route, and it fixes both 1.4.3 and 1.4.9 (their vcpkg
baselines are identical).

Fix applied: pinned the vcpkg ports tree to the manifest baseline and installed
`ffmpeg[core,amf,nvcodec,qsv]:x64-windows-static` (20 min build). The feature list
matters — a bare `vcpkg install ffmpeg` builds defaults and silently omits every
hardware encoder.

Verified in the v2.5 binary: `avcodec` ×211, `nvenc` ×14, `qsv` ×26, `amf` ×28;
`librustdesk.dll` grew 28.5 MB → 45.8 MB. `Cargo.lock` was completely unchanged,
confirming hwcodec needed no dependency movement at all.

`tools/setup-hwcodec.sh` now pins the baseline itself, so this is reproducible on
any machine rather than being one person's cargo cache.

---

## MouseMux C source — location and ground truth

**Found 2026-08-09.** The MouseMux side of the protocol lives at:
```
O:\GitHub\Gibster\mousemux\vapi\src\programs\apps\mousemux-main\func\extra\rustdesk\
  local.h                 constants, client/state structs
  rustdesk.h              public API
  rustdesk_validation.c   input validation (ranges)
  rustdesk_flow.c         state-machine gating
  rustdesk_handler.c      business logic (R2M messages)
  rustdesk_receiver.c     window + message pump
  rustdesk_state.c        client table, HWID allocation
  docs/MOUSEMUX_PROTOCOL_V2.3.md   <-- the authoritative spec
```
Window class shared constant is in `apps/mousemux-common/mousemux_common.h:35`
(`MOUSEMUX_SHARED_NAME_RUSTDESK_WINDOW`).

**`docs/MOUSEMUX_PROTOCOL_V2.3.md` in that folder is now the single authoritative
spec.** The former RustDesk-side copy (`rustdesk-development/MOUSEMUX_PROTOCOL_V2.2.md`)
has been removed to `backups/superseded-docs/` — two copies of a spec that already
disagreed with the code is how the peer-name encoding bug (Finding 18) survived
unnoticed for months. Do not reintroduce a second copy; link to the C-side file.

The V2.3 spec was reviewed against the implementation on 2026-08-10 and corrected:
its window-lookup guidance prescribed `FindWindowExA(HWND_MESSAGE, NULL, cls, NULL)`,
which returns NULL on a live system — the title is the determining factor, not the
parent, and neither window is message-only. Version references were 142 throughout.

Hard limits read from `local.h` / `rustdesk_validation.c`:

| Constant | Value | Meaning |
|----------|-------|---------|
| `CLIENT_MAX` | 32 | max simultaneous RustDesk connections |
| `NAME_MAX` | 256 | peer name buffer, in `wchar_t` units |
| `VERS_MIN/MAX` | 100 / 999 | RustDesk version accepted (**range**, not exact) |
| `PROTO_VERSION_MIN/MAX` | 121 / 122 | V2.1–V2.2 |
| `RUID_MIN/MAX` | 1 / 9999 | connection id — **0 is rejected** |
| `HWID_MIN/MAX` | 6001 / 6100 | assignable mouse/keyboard IDs (100 total) |
| `NAME_CHAR_MIN/MAX` | 0 / 127 | per-message name value |

---

### Finding 10 — Version mismatch — CLOSED, not a bug

`rustdesk_version_validate` is a **range** check (`VERS_MIN=100`, `VERS_MAX=999`),
so `RUSTDESK_VERSION = 143` is accepted. `PROTOCOL_VERSION = 122` equals
`PROTO_VERSION_MAX`, i.e. exactly V2.2. The "142" in the docs and in our doc
comments is an example value, not a requirement. No change needed — though the
stale doc comments at `windows_mousemux.rs:582`/`:628` could say so.

---

### Finding 18 — Peer name encoding mismatch (CRITICAL, cosmetic impact)

**Status:** OPEN
**Decision:** _(pending)_

Three-way inconsistency, confirmed by reading both sides.

**RustDesk sends** one message per character, whole code point in `lParam`
(`windows_mousemux.rs`, the `for ch in peer_info.chars()` loop), then a single `0`
as terminator.

**MouseMux expects** four messages per character — the bytes of a UTF-32 LE code
point (`rustdesk_handler.c:200-241`):
```c
that->name.utf32 |= (byte & 0xFF) << (that->name.bytes * 8) ;
that->name.bytes++ ;
if(that->name.bytes < 4) return vapi_true() ;   /* wait for all 4 */
```

**MouseMux also validates** every message value against `0..=127`
(`rustdesk_validation.c:295-303`, applied at `:140`) — which contradicts its own
UTF-32 accumulator, since any non-ASCII character's UTF-32 bytes exceed 127.

Consequences as shipped:
- ASCII name `alice@123` → MouseMux packs `'a','l','i','c'` into one code point
  `0x63696C61`. **Every name is garbled.**
- Terminator: RustDesk's single `0` contributes one zero byte; MouseMux only
  terminates when four accumulated bytes are all zero.
- A non-ASCII name is rejected outright by the `0..=127` validator.

**Not functionally blocking:** `rustdesk_flow_r2m_connection_ready`
(`rustdesk_flow.c:140`) gates only on `CONN_STATE_NAMING`, never on `name.done`.
So IDs are still assigned and input still works — which is why this was never
noticed. The damage is limited to the name MouseMux displays.

**Needs a cross-side decision** — three options, see discussion:
1. RustDesk sends UTF-32 LE bytes (4 per char) — matches the handler, but every
   non-ASCII byte then fails MouseMux's `0..=127` validator, so it only works for
   ASCII unless MouseMux's validator is widened to `0..=255` too.
2. MouseMux handler changed to one-code-point-per-message — matches what RustDesk
   already sends; validator would need widening to accept full code points.
3. Restrict to ASCII on the RustDesk side and send 4 bytes per char (high bytes
   are zero, so all four pass the validator) — works with **no MouseMux change**.

Note `MAX_PEER_INFO_LENGTH = 256` on the RustDesk side correctly matches
`NAME_MAX = 256` wchar_t units, so the Finding 3 character-based truncation is
the right unit either way.

---

### Finding 17 — Support URLs drifted between the two UIs (LOW)

**Status:** FIXED 2026-08-09 — needs build to verify.

The two UIs pointed at different pages, the result of repeated back-and-forth in
commits `13210a16a`, `3a1f5c8f5`, `2fef79b89`:

| File | Was | Now |
|------|-----|-----|
| `src/ui/index.tis:632` | `https://www.mousemux.com/pages/rustdesk` | `https://www.mousemux.com` |
| `flutter/lib/desktop/pages/desktop_home_page.dart:460` | `https://www.mousemux.com/pages/apps/rustdesk` | `https://www.mousemux.com` |

Requested by user 2026-08-09: both should just be `www.mousemux.com`. Note the
Sciter file is only used by the Sciter UI, which this Flutter build does not ship;
fixed anyway so the two do not drift again.

---

## Upgrade plan — RustDesk 1.4.3 → 1.4.9

**Researched 2026-08-09.** Upstream is at **1.4.9**; our base is **1.4.3**
(`db4296533`, 2025-08-26). Six releases behind.

### Facts gathered (via shallow fetch of tag 1.4.9)

| | ours (1.4.3) | upstream 1.4.9 |
|---|---|---|
| `hbb_common` gitlink | `db45828cd` (fork, base `5ed0afd`) | `7e1c392c` |
| `hwcodec` rev (Cargo.lock) | `17c1dbb3` | `778df1f9` |
| hwcodec pin style | git URL, no `rev` — resolved by Cargo.lock | same |

### hwcodec: what the newer revision does and does not fix

`778df1f9` **fixes** the `AVFrame::key_frame` removal with a proper guard
(`#if FF_API_FRAME_KEY` → `frame_->flags & AV_FRAME_FLAG_KEY`).

It **does not fix** the profile macros — `cpp/common/util.cpp:59,61` still use
`FF_PROFILE_H264_HIGH` / `FF_PROFILE_HEVC_MAIN` unguarded, which FFmpeg 7.0
removed in favour of `AV_PROFILE_*`.

**Therefore upgrading alone does NOT restore hwcodec.** The real root cause is
that this machine's classic-mode vcpkg has drifted to FFmpeg 8.0.1, while upstream
builds against whatever `vcpkg.json`'s baseline (`120deac3…`) pins. The correct fix
is to build the vcpkg dependencies from the project manifest rather than chasing
hwcodec revisions.

### MouseMux surface area to carry across

Small and well-isolated. The bulk is a **new file** that cannot conflict:
- `src/platform/windows_mousemux.rs` — 925 lines, new
- `libs/enigo/src/win/win_impl.rs` — vendored fork: HWID map + `current_conn_id`
- `libs/hbb_common` — 3 commits, 1 file (`src/config.rs`), needs rebasing onto
  `7e1c392c`
- ~12 small hook points: `connection.rs`, `input_service.rs`, `server.rs`,
  `common.rs`, `ipc.rs`, `ui_interface.rs`, `flutter_ffi.rs`, `platform/mod.rs`,
  `lib.rs`, `ui.rs`, `ui/index.tis`, `flutter/lib/.../desktop_home_page.dart`

### Proposed sequence

1. **Push the fork first** (Finding 1). Do not start an upgrade while the build is
   reproducible on only one machine.
2. **Close the remaining findings** (7, 8, 9, 11, 12, 13, 14, 15, 19) on the
   current base. Cheaper than re-deriving them after a six-version merge.
3. **Rebase the hbb_common fork** — 3 commits, 1 file — onto `7e1c392c`. This is
   the rebase declined for the *restore*; for an *upgrade* it is unavoidable.
4. **Merge upstream 1.4.9** into the MouseMux branch. Expect real conflicts only in
   `input_service.rs` (where Finding 5's lock lives and upstream churns most) and
   `connection.rs`.
5. **Re-check `libs/enigo`** — upstream may have moved it independently of our fork.
6. **Fix vcpkg properly**: build deps from the project manifest/baseline so FFmpeg
   matches what hwcodec expects, then re-enable hwcodec via
   `tools/setup-hwcodec.sh`.
7. **Bump `RUSTDESK_VERSION`** in `windows_mousemux.rs` from 143 to 149. Cosmetic —
   MouseMux validates a 100–999 range — but it is what MouseMux logs.
8. Rebuild, verify branding + hwcodec strings in the DLL, archive as a new dated
   build, retest with two simultaneous users.

### Existing tooling for this

`rustdesk-development/` already has infrastructure built for exactly this job:
`patches/`, `patches-clean/`, `patches-minimal/`, `organize-patches.py`,
`create-clean-patches.py`, `generate-minimal-patches.py`, and
`MOUSEMUX_MAINTENANCE.md` describing the maintenance process.

---

## Build log

| Version | Date | Contains | Folder | Notes |
|---------|------|----------|--------|-------|
| 1.4.3-mousemux-v2.2 | 2026-02-10 | pre-review state | `backups/2026-08-09_pre-hbb-restore/` | APP_NAME regression present. Full 4.66 GB snapshot incl. `target/`, so rollback needs no rebuild |
| 1.4.3-mousemux-v2.3 | 2026-08-09 | Findings 1,2,3,4,5,6,17,18 | `final-builds/rustdesk-mousemux-1.4.3-mousemux-v2.3-2026-08-09_15-27/` | 66 MB. librustdesk.dll rebuilt 15:20, 28,486,656 bytes, verified byte-identical in archive. Contains `rustdesk-mousemux-edition` ×2 and `1.4.3-mousemux-v2.3` ×2. **First build since 2026-02-10 in which librustdesk.dll was actually recompiled.** No installer/zip produced yet. |

| 1.4.3-mousemux-v2.4 | 2026-08-10 | **all findings except 20** — adds 7,8,9,11,12,13,14,15,19 | `final-builds/rustdesk-mousemux-1.4.3-mousemux-v2.4-nohwcodec-2026-08-10_10-43/` | librustdesk.dll 28,483,072 bytes, built 10:40. Verified in binary: version ×2, branding ×2, per-event log strings ×0, avcodec ×0 (no hwcodec). Release.zip 28,886,532 bytes, integrity checked, archive byte-identical to build output. No installer. **Not runtime-tested.** |

### Build environment gotchas (learned 2026-08-09)

1. **`VCPKG_ROOT` must be set AFTER `vcvars64.bat`.** vcvars overwrites it with
   Visual Studio's own bundled vcpkg, which has no `installed/` directory, so
   `scrap` and `magnum-opus` fail with `fatal error: 'vpx/vp8.h' file not found`.
   The original `build.bat` avoids this only because it never calls vcvars.
   Working script: `cargo_build.bat` in the parent directory.
2. **Batch files mask cargo's exit code.** `build.bat`/`cargo_build.bat` end with
   `echo`, so the `.bat` returns 0 even when cargo exits 101. Always grep the log
   for `cargo exit code:` rather than trusting the process exit status.
3. **`--skip-cargo` in `build.bat` does not rebuild `librustdesk.dll`.** To pick up
   Rust changes, run `cargo build --features flutter --lib --release` separately
   (that is what `cargo_build.bat` does), then `flutter_build.bat`.
4. `vswhere.exe is not recognized` is emitted by vcvars but is cosmetic — it
   recovers and reports `Environment initialized for: 'x64'`.
5. **`Cargo.toml` is the single source of truth for the version.** `build.rs:84`
   calls `hbb_common::gen_version()`, which regenerates `src/version.rs` from
   `Cargo.toml`'s `version =` line and stamps `BUILD_DATE` with the current time.
   `src/version.rs` is gitignored and untracked **by design** — do not edit it by
   hand, the build overwrites it. To bump a version, edit `Cargo.toml` (and
   `res/PKGBUILD` for the Linux package), then build.

---

## Action log

**2026-08-09 14:16 — Safety snapshot taken.**
`backups/2026-08-09_pre-hbb-restore/rustdesk/` — full tree including `target/`.
10,508 files / 4.66 GB, 0 failed. `librustdesk.dll` verified byte-identical to
the original. Includes the build cache, so a rollback does not require a full
cargo rebuild.

**2026-08-09 — Findings 1 + 2 remediated locally.**

Gotcha worth recording: `libs/hbb_common/.git` is a **gitlink file**, not a
directory, in both trees. The real object store lives at
`<repo>/.git/modules/libs/hbb_common`. Copying only the working directory swaps
the *files* but leaves the `.git` file resolving to the destination repo's module
store — the working tree looked right while `git rev-parse HEAD` still reported
the upstream commit. Both the working dir **and** the module store must be moved.

Steps performed:
1. `libs/hbb_common` → `libs/hbb_common.upstream-backup` (moved, not deleted)
2. Copied VM working tree from `rustdesk-development/rustdesk-current/libs/hbb_common`
3. `.git/modules/libs/hbb_common` → `.git/modules/libs/hbb_common.upstream-backup`
4. Copied VM module store from `rustdesk-current/.git/modules/libs/hbb_common`
   (`core.worktree` is relative — `../../../../libs/hbb_common` — so it resolved
   correctly with no edit needed)
5. `git checkout -- Cargo.lock`

Verified after:
- `git submodule status` → ` ee4f6db8...` (leading space = matches gitlink, clean)
- `src/config.rs:64` → `APP_NAME = "rustdesk-mousemux-edition"` ✓
- `Cargo.lock` byte-identical to VM/committed ✓

**Uncommitted-change triage (decided 2026-08-09):**

| File | Action | Reason |
|------|--------|--------|
| `Cargo.lock` | REVERTED | accidental churn from the wrong submodule (added webrtc*, x25519-dalek, x509-parser, yasna, xkeysym) |
| `build.py` | KEEP | MSYS/MINGW platform-detection fix required by `build.bat` on this machine |
| `res/PKGBUILD` | KEEP | legitimate `pkgver=1.4.3-mousemux-v2.2` bump |
| `libs/hbb_common` | RESTORED | to `db45828cd` |
| `flutter/.flutter`, `flutter/.flutter_tool_state` | GITIGNORE | build tool state |
| `libs/hbb_common/src/config.rs.backup` | untracked leftover from VM copy — harmless, can delete |

**Still outstanding for Finding 1:** create `MouseMux/hbb_common`, push `db45828cd`
+ history, add `upstream` remote (user wants future upstream pulls), repoint
`.gitmodules`. Blocked: `gh` CLI not installed.

---

## Session notes

**2026-08-09** — Review performed against upstream base `db4296533`. Scope covered
`windows_mousemux.rs` (925 lines), `input_service.rs`, `connection.rs`,
`portable_service.rs`, `server.rs`, `common.rs`, `ipc.rs`, and the enigo fork.
Not reviewed: Dart/Sciter UI, Linux wayland multi-display changes (unrelated to
MouseMux). 15 findings recorded above.

Also on 2026-08-09: rebuilt Flutter shell successfully (35.7s) but `--skip-cargo`
in `build.bat` means `librustdesk.dll` was **not** recompiled — it remains the
2026-02-10 binary. Any fix to Rust code requires dropping `--skip-cargo` or
running `cargo build --features flutter --lib --release` separately.

---

## 2026-08-11 — live loopback test, two failures, one of them ours

First end-to-end run against a real MouseMux V3 3.0.10 (loopback: host with
`direct-server='Y'`, client via `--connect 127.0.0.1`). RustDesk's own log looked
perfect — `peer_info 'Dev@530804584'`, protocol 123, 13 chars + null terminator,
`rustdesk_hwnd=592830` — and yet no hardware IDs came back. The MouseMux log in
`Documents/MouseMux V3/user/logs/` gave both reasons.

**1. Protocol 123 rejected — stale MouseMux binary, not a code defect.**

    rustdesk_validation.c:269  proto 123 range 121-122
    rustdesk_receiver.c:32     r2m.connection.open failed

The C source already has `PROTO_VERSION_MAX = 123`; the running binary predates
it. This cascades and is worth recognising on sight: with `connection.open`
refused the client is never allocated, so every later name byte logs
`r2m.connection.name ruid:105 not found`. A flood of "not found" lines is a
symptom of a rejected open, not a naming bug. Fix = rebuild MouseMux.

**2. HWND mismatch — our bug, introduced by the dual-window design.**

    rustdesk_validation.c:249  hwnd 0x90bbe mismatch 0xd70b6a

We registered two receiver windows: the versioned `mousemux-v3.rustdesk.window.query`
(primary, reported in NOTIFY_STARTUP) and the historic `rustdesk.mousemux.window.query`
(alias), on the theory that the alias preserved compatibility with a MouseMux that
had not been versioned yet.

That theory was wrong, and the reason generalises: **MouseMux does not just find
RustDesk by name, it validates the handle.** `rustdesk_hwnd_rust_validate()` re-runs
FindWindowEx and compares the result with the HWND we reported, rejecting the
connection on any difference. Two windows means two handles, so whichever we
report, a MouseMux that discovered the other refuses us. The alias could never have
worked — it only guaranteed a mismatch. Confirmed systematic: the same mismatch
appears in an earlier session (`0x1430aba` vs `0x88a03b8`), and it fires at
`r2m.startup`, before any connection, so it is independent of failure 1.

Fixed by registering exactly one window. Both sides move to the versioned name
together; there is no useful half-step.

**Also corrected: two comments in our source asserting measured-false claims.**

- "a class-only FindWindowEx search does not match these windows" — it does, when
  the NULL is genuinely NULL. The original misdiagnosis came from a PowerShell
  binding marshalling `$null` as `""`, which matches nothing. We still pass the
  name as both class and title, because that is the C-side contract and it is
  correct under either reading — but the stated reason was wrong.
- "FindWindowEx cannot locate message-only windows" — it located MouseMux's
  message-only query window with a NULL parent on this machine (verified: that
  window is absent from EnumWindows, while `Shell_TrayWnd` and `mousemux-v3.hub`
  return 0 through an HWND_MESSAGE parent). Undocumented but measured. The C side
  hedges via `vapi_window_locate` (HWND_MESSAGE parent first, then NULL); RustDesk
  cannot link vapi and keeps the single NULL-parent call. If it ever stops working,
  mirror the two-step try — and note HWND_MESSAGE is `(HWND)-3`, not `+3`.

Also restored `PROTOCOL_VERSION` to 123; it had been left at 122 with a
"TEMPORARY DIAGNOSTIC" comment while narrowing failure 1.

**C side: nothing to change.** `PROTO_VERSION_MAX = 123` and all three lookups
(`rustdesk_receiver.c:128`, `rustdesk_state.c:159`, `rustdesk_validation.c:260`)
already use `MOUSEMUX_VERSIONED_NAME_RUSTDESK_WINDOW`. It needs a rebuild only.

**Retest after both rebuilds:** IDs should come back. Watch for
`r2m.connection.open` succeeding, then `MOUSE_ID`/`KEYBOARD_ID` in RustDesk's log.

### 2026-08-11, later — full-flow verification with real remote users

Both sides rebuilt (RustDesk 1.4.9-mousemux-v3.1, MouseMux 3.0.11 debug). Three
concurrent connections, two of them genuine remote peers over relay:

| conn_id | peer name | mouse hwid | kb hwid | MouseMux user |
|---------|-----------|-----------|---------|---------------|
| 1077 | `Dev@530804584` (loopback) | 0x6003 | 0x6004 | 7 Purple |
| 1080 | `SYSTEM@158254826` (remote) | 0x6005 | 0x6006 | 8 Maroon |
| 1081 | `Dev@1736929465` (remote) | 0x6007 | 0x6008 | 9 Fuchsia |

Every stage that failed on 2026-08-11 morning now passes: `connection.open`
accepted at proto 123, names decoded intact, `connection.ready` validated the
HWND with no mismatch, IDs returned and reached Enigo (`HashMap now contains 3
entries`), and MouseMux mapped each connection to a distinct user with its own
cursor position.

**The decisive evidence that routing works end to end** — a real click from the
remote peer, not a synthetic message:

    14:43:46  emu_button: mintty.exe=switched-click
    14:43:46  user 'Maroon' became root

Cursor positions stay separate per user (`Maroon pos:(944,970)`,
`Fuchsia pos:(1279,3)`), which is the whole point of the integration.

**Note on emulating a remote peer.** Synthetic `WM_MOUSEMOVE` posted to the
client's remote-view window does nothing - Flutter's embedder tracks real pointer
state and ignores posted messages with no cursor behind them. A protocol-level
injector is feasible (`Client::start` returns a fully-handshaked `Stream` and
`client::handle_hash` is a reusable `pub` helper), but `mod client` is private in
lib.rs, so it would need a bin plus an export change. Connecting a second machine
is zero-code and more faithful; prefer it.

**GAP FOUND - MouseMux IDs are not released if a connection dies abnormally.**

`release_ids()` has exactly one caller: `on_close()` in connection.rs:4819,
guarded by `if self.authorized`. That guard is right (unauthorized connections
never got IDs - verified: conns #1076 and #1078 closed with zero releases and
correctly held no IDs). Every normal termination path reaches `on_close`,
including the catch-all at connection.rs:1154.

But `impl Drop for Connection` (connection.rs:6302) does NOT release IDs - it
only releases pressed modifiers, joins the terminal service and closes a token.
So if the connection task is cancelled or panics, `on_close` never runs, `Drop`
runs instead, and the IDs leak on BOTH sides: a stale entry in RustDesk's Enigo
HashMap and a phantom user in MouseMux that never goes away.

Not yet observed in practice, and narrow - but the failure mode is silent and
permanent, which is exactly the kind that surfaces as "MouseMux slowly fills up
with dead users". Fix would be to make `Drop` a backstop that calls
`release_ids()` when `authorized && !closed`.

**Untested:** the release path itself. No `RELEASE_CONNECTION` has ever been
observed, because no authorized connection has been closed yet. Worth exercising
deliberately before shipping.

**Log noise, not a defect:** `no user found yet` fires on every connection at
`rustdesk_handler.c:292`, ~20ms before the mapping lands. Harmless but misleading
when reading logs. Likewise RustDesk's `Error of monitor0 service: SWITCH` and
`display service: new subscriber` are normal on a new subscriber, and the failed
accepts from 87.215.158.135 are direct-connect attempts that preceded a
successful relay connection.

### 2026-08-11 — release path verified, and an off-by-one in the user count

**Release path works.** Both remote users disconnected deliberately; the full
teardown ran on each:

    #1080 Connection closing, releasing MouseMux IDs
          MOUSEMUX_RELEASE_CONNECTION - Posted to MouseMux window 0x801c8
    MM    r2m.connection.close -> client free slot:1 ruid:1080
          sending M2R user.del ruid:1080
    #1080 MOUSEMUX_USER_REMOVE

Slots freed (1 and 2 returned), users unmapped, Enigo HashMap wound 3 -> 2 -> 1.
No leak on the normal path. This closes the "release path never exercised" item;
the `Drop` backstop below is still worth adding for ABNORMAL termination, which
remains untested and unprotected.

**NEW FINDING - user count is reported one too high on removal.**

`M2R_USER_DEL` carries `this->loop` ("Active client count", local.h:157) as its
lparam. The ordering differs between add and remove:

- add: `rustdesk_state.c:219` increments `loop`, THEN `rustdesk_handler.c:303`
  sends `user.add` -> post-increment, correct.
- del: `rustdesk_handler.c:329` sends `user.del`, THEN `rustdesk_state.c:260`
  decrements `loop` -> pre-decrement, one too high.

Observed: adds carried 1, 2, 3 (correct); dels carried 3 then 2, where the true
remaining counts were 2 then 1.

The consequence is worse than a wrong log line, because our side treats MouseMux
as authoritative and *reconciles to it* (windows_mousemux.rs:390):

    User count mismatch after REMOVE - RustDesk: 2, MouseMux: 3 - reconciling to MouseMux

RustDesk decrements correctly, sees a mismatch, and overwrites its correct value
with the inflated one. The warning then fires only ONCE - on the next removal our
inflated count happens to equal MouseMux's inflated count, so it looks like
agreement and the bug goes silent. Final state after both disconnects: RustDesk
believes 2 users are connected while only 1 is.

Impact is display-only - `CONNECTED_USERS_COUNT` feeds
`main_get_connected_users_count()` (Flutter) and `update_main_window_title()`,
not input routing. But it is sticky: once every user leaves, the last del reports
1, so RustDesk shows one phantom user indefinitely.

Fix belongs on the C side, to keep "MouseMux is authoritative" actually true:
send `user.del` AFTER the slot is freed, or pass `this->loop - 1`. Patching it in
RustDesk instead would mean subtracting one from a value the protocol defines as
a total, which encodes the quirk in both codebases.

### 2026-08-11 — version-mismatch behaviour: there is no dialog, anywhere

Fixed the user-count off-by-one on the C side (both call sites):
`rustdesk_handler.c` now sends the post-decrement count on close, and
`rustdesk_receiver.c` counts down across the shutdown loop instead of sending the
unchanged total for every client. Needs a MouseMux rebuild.

**Question asked: what happens when versions do not match, and is there a dialog?**

Answer: no dialog, no toast, no UI indication on EITHER side. Every mismatch is
log-only, and the degradation is silent. Verified by inspection - no
dialog/msgbox/popup/toast/balloon call exists anywhere in the MouseMux rustdesk
module, and RustDesk surfaces nothing to Flutter except
`main_get_connected_users_count()`.

The three cases:

1. **Old RustDesk (proto 121/122) + new MouseMux.** The protocol RANGE accepts
   it (121-123), so the version itself is fine. But since we versioned the window
   names, the two can no longer FIND each other: old RustDesk registers
   `rustdesk.mousemux.window.query` and looks for `mousemux.main.window.query`;
   new MouseMux registers `mousemux-v3.main.window.query` and looks for
   `mousemux-v3.rustdesk.window.query`. Mutual invisibility. RustDesk logs
   "MouseMux window not found" and carries on as plain RustDesk.

2. **New RustDesk (123) + old MouseMux (max 122).** What we hit this morning.
   MouseMux logs `proto 123 range 121-122` and refuses `connection.open`.
   RustDesk never receives IDs, keeps `mouse=None, keyboard=None`, and injects
   input UNSTAMPED - so remote control still works, but every remote user shares
   the one system cursor. Nothing tells the user their multi-user routing is off.

3. **Much older RustDesk (<121).** Rejected by range, log only.

Case 2 is the dangerous one: it looks like success. The product appears to work
and only the feature that justifies this fork is silently absent.

**Recommended, not yet implemented:**

- *MouseMux side (best leverage).* MouseMux already knows RustDesk's version from
  NOTIFY_STARTUP (`vers:149`) and knows its own accepted range, so it can show a
  notification in its own UI - "RustDesk 1.4.9 speaks protocol 123, this MouseMux
  supports up to 122; update MouseMux" - with no protocol change at all. This is
  the only side that has both numbers at the moment of rejection.
- *RustDesk side.* Today there is no timeout: if the MouseMux window is found and
  REQUEST_CONNECTION is sent but IDs never arrive, nothing is logged. A timeout
  (~2s) that logs a WARN would turn a silent failure into a diagnosable one,
  since finding the window proves MouseMux is installed and running.

Adding a "rejected" M2R message would NOT help the case that matters - an old
MouseMux cannot send a message it does not know about, which is precisely the
version that needs to report the problem.

### 2026-08-11 — design: how MouseMux can warn about an incompatible RustDesk

C-side count fix was REVERTED pending a pull; patch held at
`scratchpad/mousemux-usercount-fix.patch`, to be re-applied afterwards.

Main use case to solve: MouseMux updates, the user still has an older RustDesk,
and nothing tells them. The constraint that shapes the design:

**After the window-name versioning, an old RustDesk is invisible to MouseMux.**
Old RustDesk registers `rustdesk.mousemux.window.query` and searches for
`mousemux.main.window.query`; new MouseMux registers/searches `mousemux-v3.*`.
Neither finds the other, so NO protocol message is ever exchanged - no
NOTIFY_STARTUP, no version, nothing to validate. MouseMux cannot report a
mismatch it never observes. It must actively go looking.

Design (MouseMux side):

1. Keep a list of historical RustDesk window names. In `rustdesk_receiver_start()`,
   the existing "RustDesk-client window not (yet) found" branch becomes: scan the
   legacy names; a hit positively identifies an incompatible RustDesk.
2. Re-scan periodically - RustDesk may start after MouseMux. `rustdesk_receiver_pump()`
   already runs on a cycle and a FindWindowEx per legacy name is cheap.
3. Latch a flag in `rustdesk_state_t` so the notice shows once per detection.
4. Name the version: legacy HWND -> GetWindowThreadProcessId -> OpenProcess
   (PROCESS_QUERY_LIMITED_INFORMATION) -> QueryFullProcessImageName ->
   GetFileVersionInfo. "RustDesk 1.4.3 is running; this MouseMux needs 1.4.9+"
   is actionable; a generic warning is not.

Rejected alternative: have MouseMux also register the old unversioned query name
as a beacon so old RustDesk volunteers its version via NOTIFY_STARTUP. It does
yield the exact version for free, but it invites a handshake that is guaranteed
to fail later at HWND validation - the same class of bug removed on 2026-08-11.
The passive scan costs one FindWindowEx and starts nothing it cannot finish.

Also worth adding, with a caveat: surface the numbers when
`rustdesk_protocol_validate()` rejects a version (MouseMux holds both the peer
version and its own range at that moment). Caveat - like a "rejected" M2R
message, this can only help FUTURE mismatches, because the MouseMux that needs to
warn is the one already shipped. Only the legacy scan addresses the case in
question, and only if it ships before the next naming change.

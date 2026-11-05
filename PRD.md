# Monk Mode Blocker — Product Requirements Document (PRD)

> **Version:** 1.1 · **Date:** 2026-11-05 · **Owner:** tokiosman96 · **Builder:** Claude Code (the whole app is implemented by Claude Code, task by task, from this file)
>
> **Status:** not started. The project is built from scratch; there is no earlier code to port.

---

## 0. How Claude Code should use this file

1. Work through **Section 9 (Task list)** top to bottom. Do not skip phases.
2. Before starting a task, read its **Acceptance** line. A task is done only when the acceptance check passes.
3. When a task is done, change `- [ ]` to `- [x]` in this file **in the same commit** as the code.
4. If a task turns out to be wrong or impossible, don't silently skip it: add a note under it (`> NOTE: ...`), and add a replacement task.
5. Keep the UI simple (see Section 6). Do not add features that are not in this PRD; put ideas in **Section 11 (Backlog)** instead.
6. After each phase, run `npm run lint`, `npm run test` and `cargo test --workspace` and fix failures before moving on.

---

## 1. Problem & goal

The user is doing **3 months of "monk mode"**: 12 hours of study a day:

| Block | Hours/day |
|---|---|
| Learning React | 3 |
| Building products & projects | 6 |
| LinkedIn Learning (marketing course) | 2 |
| Buffer / breaks | 1 |

Social media, YouTube and music are the main distractions. On the phone, **Kahf Guard** (DNS filtering + Reels/Shorts blocking + strict uninstall protection) solves this. On the desktop there is no free tool that combines all three in a way that can't be undone on a bad day; Cold Turkey Pro costs $45.

**Goal:** a free desktop app that:
- gives a **small daily allowance** (default 10 min) for distracting sites, then **hard-blocks** them until midnight;
- **permanently blocks** the worst sites (TikTok);
- **cannot be weakened** before a chosen commitment end date (e.g. 3 months);
- helps the user **follow the 12-hour study plan** and see their streak.

---

## 2. Target platform

- **Windows 10/11 (x64) only.** No macOS or Linux support is planned. No cross-platform stubs or `cfg` fallbacks are needed.
- Development, tests and builds run on **Windows** (the owner's PC, or the `windows-latest` GitHub Actions runner).
- Browsers supported by the extension: **Chrome, Edge, Brave** (Chromium, Manifest V3).

---

## 3. Inspiration: how Kahf Guard works → how we copy it on desktop

| Kahf Guard (mobile) | Monk Mode Blocker (desktop) |
|---|---|
| Filtering DNS server answers "blocked" for bad domains | **Hosts file** entries `0.0.0.0 domain` for blocked domains (works in every browser and app) |
| Reels / Shorts blocking inside apps | **Browser extension** blocks `/shorts`, `/reels`, `/reel/` URLs and hides feed links |
| Screen time limits | **Browser extension** measures real time on sites |
| Strict mode / uninstall protection | **Commitment lock** + background **Windows Service** that re-applies rules and restarts the app |

---

## 4. Features

### F1. Daily allowance (core)
- One shared **daily allowance** for all "allowance" sites. Default **10 minutes**, configurable 0–60.
- Time is counted **only while an allowance site is the active, focused, visible tab** (measured by the extension), not while it's open in the background or while the PC is idle/locked.
- When the allowance hits 0 → all allowance sites are added to the hosts file and the extension shows the block page. They stay blocked until **local midnight**, then the allowance resets.
- Warnings: notification at **2 min left** and **30 s left**.
- Default allowance sites: `youtube.com, facebook.com, instagram.com, x.com, twitter.com, reddit.com, open.spotify.com, music.youtube.com, soundcloud.com`.

### F2. Always-blocked list
- Sites blocked **24/7** with no allowance, through the hosts file.
- Default: **`tiktok.com`** (plus `tiktokv.com`, `tiktokcdn.com`). The user can add more.

### F3. Shorts / Reels blocker
- Always on (can be turned off only when not locked).
- Blocks: `youtube.com/shorts/*`, `instagram.com/reels/*`, `instagram.com/reel/*`, `facebook.com/reel/*`, `facebook.com/watch*`.
- Hides Shorts shelves/links on YouTube home & search with a small content script (CSS `display:none` on known selectors). Must not break normal video watching.
- Shorts/Reels are blocked even while daily allowance is left.

### F4. Browser protection
- If a supported browser is running **without the extension connected** for > 30 s, the service fully blocks all allowance domains in the hosts file (the extension is what counts time; without it, no allowance is given).
- Incognito: the setup wizard tells the user to enable "Allow in Incognito". If incognito tabs are detected without the extension, same rule.

### F5. Commitment lock (strict mode)
- User picks an end date (presets: 1 week, 1 month, **3 months**, custom).
- While locked, the user **can only make rules stricter**:
  - **Allowed:** add allowance sites, add always-blocked sites, move a site from allowance to always-blocked, lower the daily allowance.
  - **Forbidden:** remove any site, move a site from always-blocked to allowance, raise the allowance, turn off the Shorts/Reels blocker, remove or shorten the lock, stop/disable the service, uninstall.
- Confirmation dialog before locking with a typed phrase: `I commit to monk mode`.
- Lock end time is stored in the database **and** a signed copy in the Windows registry (`HKLM\SOFTWARE\MonkMode`), so deleting one doesn't unlock.
- Changing the system clock backwards must not shorten the lock: the service stores the last-seen time and treats `max(now, lastSeen)` as "now".

### F6. Tamper protection
- A **Windows Service** (`MonkModeService`, runs as LocalSystem, auto-start, recovery = restart on failure) owns all rules and enforcement. The UI is just a client.
- Every **5 s** the service re-checks the hosts file block and rewrites it if changed.
- If the UI (tray app) is killed, the service restarts it.
- While locked: the uninstaller refuses to run (it asks the service, and the service says no until the lock date). Service stop requests from the UI are refused.
- Honest limitation (show in About screen): an admin user who boots into Safe Mode or reinstalls Windows can get around this. The goal is to make giving up **slow and annoying**, not impossible.

### F7. Study planner & Pomodoro
- Daily plan with 3 default blocks: **React 3h, Building 6h, LinkedIn course 2h** (editable names and hours).
- Pomodoro timer (default 50 min focus / 10 min break, configurable). User picks which block a session counts towards.
- While a Pomodoro focus session runs, **all allowance sites are fully blocked** (allowance can't be spent during a session).
- Today view shows a progress bar per block, e.g. `React 1h 40m / 3h`.

### F8. Stats & streak
- Per day: focus time per block, allowance used, number of blocked attempts.
- **Streak**: consecutive days where total focus ≥ the daily target (default 11h; configurable).
- "Day X of 90" counter while a commitment lock is active.
- Simple 7-day and 30-day bar charts (plain SVG, no chart library).

### F9. Tray & notifications
- Runs in the system tray. Tray menu: Open, Start Pomodoro, Remaining allowance (read-only text), Quit (Quit only closes the UI; the service keeps running).
- Starts on login.

### F10. Block page
- Shown by the extension when a site is blocked. Content: big "Back to work." message, today's progress, current streak, and a button "Open my plan" (opens the app). No "unblock" button.

---

## 5. Architecture

```
┌─────────────────────────────── Windows ──────────────────────────────────┐
│                                                                          │
│  monk-service.exe  (Windows Service, LocalSystem, Rust)                  │
│   ├─ rules engine (allowance, always-blocked, pomodoro, lock)            │
│   ├─ hosts-file enforcer (every 5 s)                                     │
│   ├─ SQLite db  C:\ProgramData\MonkMode\monk.db                          │
│   ├─ WebSocket server  127.0.0.1:47613  (extension)                      │
│   └─ Named pipe  \\.\pipe\monkmode  (Tauri UI)                           │
│            ▲                               ▲                             │
│            │ JSON messages                 │ JSON messages               │
│  ┌─────────┴──────────┐        ┌───────────┴────────────┐                │
│  │ Browser extension  │        │ Tauri app (tray + UI)  │                │
│  │ MV3, TypeScript    │        │ React + DaisyUI        │                │
│  │ - active-tab timer │        │ - settings, planner    │                │
│  │ - Shorts/Reels     │        │ - stats, lock wizard   │                │
│  │ - block page       │        │ - Rust commands talk   │                │
│  └────────────────────┘        │   to service via pipe  │                │
│                                └────────────────────────┘                │
└──────────────────────────────────────────────────────────────────────────┘
```

Key rule: **the service is the only source of truth**. The UI and extension never write rules or usage directly; they send requests and the service validates them (e.g. rejects loosening while locked).

### 5.1 Repo layout (Cargo workspace + npm workspaces)

```
monk-mode-blocker/
├─ PRD.md
├─ Cargo.toml                 # workspace: crates/*, app/src-tauri
├─ package.json               # npm workspaces: app, extension
├─ crates/
│  ├─ core/                   # pure Rust: rules engine, models, validation. NO OS calls. Fully unit-tested.
│  ├─ platform/               # Windows stuff: hosts file, browser detection, registry
│  └─ service/                # monk-service binary: loops, db, ws server, named pipe, service install
├─ app/                       # Tauri 2 app
│  ├─ src/                    # React + TS + DaisyUI
│  └─ src-tauri/              # Tauri Rust side: commands -> named pipe client
├─ extension/                 # MV3 extension (TypeScript, built with Vite)
├─ installer/                 # NSIS hooks for service install/uninstall
├─ docs/                      # manual tests, notes
└─ .github/workflows/ci.yml
```

### 5.2 Protocol (shared by pipe and WebSocket)

JSON, one message per frame: `{ "id": "uuid", "type": "...", "payload": {...} }` → reply `{ "id": "...", "ok": true, "data": {...} }` or `{ "id": "...", "ok": false, "error": { "code": "LOCKED_LOOSENING", "message": "..." } }`.

Message types (v1): `get_state`, `update_settings`, `start_lock`, `start_pomodoro`, `stop_pomodoro`, `get_stats`, `ext_hello`, `ext_heartbeat`, `ext_active_tab` (url, focused, visible, incognito), `ext_blocked_attempt`, `state_changed` (server push).

Types are defined once in `crates/core` and exported to TypeScript with **`ts-rs`** into `app/src/types/` and `extension/src/types/` (generated, do not edit by hand).

### 5.3 Data model (SQLite via `rusqlite`, migrations in `crates/service/migrations/`)

- `settings` (single row JSON blob, versioned)
- `lock` (`ends_at`, `created_at`, `signature`)
- `usage_daily` (`date`, `allowance_used_sec`, `blocked_attempts`)
- `focus_sessions` (`id`, `block_name`, `started_at`, `ended_at`, `kind` = pomodoro|manual)
- `meta` (`last_seen_time`, schema version)

---

## 6. UI spec (keep it simple)

- **Stack:** React 18 + TypeScript + Vite + **Tailwind CSS + DaisyUI** (DaisyUI is a component plugin on top of Tailwind; we only use its ready-made classes like `btn`, `card`, `stat`, `progress`, `toggle`, `modal`. **No custom CSS** beyond the Tailwind config).
- Theme: two custom DaisyUI themes, `monk-light` and `monk-dark` (see **6.1 Design system**). Follows the Windows light/dark setting.
- Icons: `lucide-react`. No other UI libraries.
- Window: 900×640, resizable, min 720×520.
- Navigation: left sidebar (DaisyUI `menu`) with 5 screens:

| Screen | Contents |
|---|---|
| **Today** | Allowance left (big `stat`), 3 progress bars for study blocks, Start Pomodoro button, streak + "Day X of 90" |
| **Block lists** | Tabs: Allowance sites · Always blocked. Each: list + add input + remove button (remove disabled with tooltip "Locked until …" when locked) |
| **Schedule** | Daily allowance minutes, Pomodoro lengths, study blocks (name + hours), daily focus target |
| **Stats** | 7/30 day bars, totals per study block, blocked attempts |
| **Lock** | Status, or the 3-step lock wizard (choose duration → review rules → type phrase) |

- First-run **Setup wizard**: 1) welcome, 2) install/verify service (UAC prompt), 3) install extension (unpacked instructions + "Waiting for extension…" check), 4) confirm default lists, 5) optional: start 3-month lock.
- All loosening controls must be **visibly disabled** when locked, not just rejected by the service.

### 6.1 Design system: warm, calm, minimal

**Feel:** a quiet study room. Warm paper background, one muted orange accent, lots of whitespace, slow and soft motion. It is inspired by the calm style of modern AI chat apps, but **everything is original**: do not copy any company's logo, wordmark, icon or exact brand colors.

**Colors** (DaisyUI theme tokens)

| Token | `monk-light` | `monk-dark` | Use |
|---|---|---|---|
| `base-100` | `#FAF8F3` | `#1F1E1B` | App background |
| `base-200` | `#F2EEE6` | `#2A2925` | Sidebar, cards |
| `base-300` | `#E6E0D4` | `#3A3833` | Borders, dividers |
| `base-content` | `#2B2A27` | `#ECE8DF` | Main text |
| `primary` | `#C8643B` | `#D97A52` | Buttons, active nav, progress, logo |
| `primary-content` | `#FFFFFF` | `#1F1E1B` | Text on primary |
| `secondary` | `#7A8B6F` | `#93A487` | Study-block progress (sage green) |
| `neutral` | `#6B675E` | `#A39E93` | Muted text, icons |
| `success` / `warning` / `error` | `#5E8C61` / `#D9A441` / `#B5473A` | `#78A57B` / `#E2B65C` / `#CF6456` | Status |

Text/background pairs used for body text must pass WCAG AA (4.5:1); a unit test checks them.

**Typography**
- Headings and big numbers (allowance timer, streak): **Source Serif 4**, weight 500–600.
- Body/UI: **Inter**, 14–15 px, line-height 1.6.
- Fonts bundled locally with `@fontsource` (no CDN).

**Shape & spacing**
- Radius: box `1rem`, button `0.75rem`, badge `1.5rem`.
- Cards: `base-200` background, 1px `base-300` border, `shadow-sm` max.
- Generous padding (`p-6` cards, `gap-6` layout). Max content width 760 px, centered.

**Logo (original)**
- Mark: a simple **hourglass inside a circle**, line style, 2 strokes in `primary`, a small dot of "sand" at the bottom. No gradients.
- Wordmark: "Monk Mode" in Source Serif 4.
- `app/src/assets/logo.svg` (uses `currentColor`), plus icons generated for Tauri (`npm run tauri icon`) and the extension (16/32/48/128 px).

**Motion** (CSS keyframes in Tailwind config; no animation library)
- `breathe`: scale 1 → 1.04 → 1 and opacity 0.85 → 1, **4 s**, ease-in-out, infinite. On the logo while a Pomodoro runs, and on the block page.
- `fade-up`: screens and cards fade in and move 6 px up, **250 ms**, ease-out, 40 ms stagger for lists.
- Progress bars animate width changes over **600 ms**.
- Allowance countdown: digits cross-fade (150 ms) instead of jumping.
- Buttons: 150 ms color transition only.
- `prefers-reduced-motion`: all animations off.

**Voice (UI text):** short, warm, calm. Examples: "Back to work." · "8 minutes left for today." · "Day 12 of 90. Keep going." No exclamation marks, no guilt-tripping.

---

## 7. Tech stack

| Area | Choice |
|---|---|
| Desktop shell | **Tauri 2.x** (`tauri-plugin-notification`, `tauri-plugin-autostart`, `tauri-plugin-single-instance`) |
| Frontend | React 18, TypeScript 5, Vite 5, Tailwind 3 + **DaisyUI 4**, lucide-react, Zustand, React Router, `@fontsource/source-serif-4`, `@fontsource/inter` |
| Rust | edition 2021, `tokio`, `serde`, `rusqlite` (bundled), `tokio-tungstenite`, `windows-service`, `windows` crate (registry, named pipes, user-session launch), `sysinfo` (browser detection only), `chrono`, `ts-rs`, `thiserror`, `tracing` |
| Extension | TypeScript, Manifest V3, Vite build, `chrome.declarativeNetRequest`, service worker + content script |
| Tests | Rust: `cargo test`; Frontend + extension: Vitest + Testing Library |
| Lint/format | `cargo clippy -D warnings`, `rustfmt`, ESLint, Prettier |
| Installer | Tauri NSIS bundle + custom NSIS hooks (service install/uninstall, admin) |
| CI | GitHub Actions: single `windows-latest` job (lint, all tests, full build) |

---

## 8. Success criteria (v1)

- With allowance = 10 min, watching YouTube in the active tab for 10 min blocks youtube.com in Chrome, Edge **and** any other app within **5 s**, until midnight.
- Background YouTube tabs do not use up allowance.
- `tiktok.com` is blocked at all times, from the first minute after install.
- `youtube.com/shorts/...` is blocked at all times; a normal `youtube.com/watch` video plays while allowance is left.
- During a Pomodoro, allowance sites are blocked even with allowance left.
- Desktop apps (Spotify, Discord, etc.) are **not** blocked or killed.
- Manually editing the hosts file to remove entries is reverted within 5 s.
- While locked: removing a site, raising allowance, stopping the service and uninstalling all fail with a clear message.
- Turning the clock back 1 month does not end a 3-month lock early.
- Killing the tray app → it's back within 10 s.
- Idle CPU < 1%, RAM (service + UI) < 120 MB.

---

## 9. Task list

### Phase 0 — Project setup
- [x] **T0.1** Init git repo, `.gitignore` (Rust, Node, Tauri), `README.md` stub, MIT `LICENSE`. *Acceptance:* first commit exists.
- [x] **T0.2** Create Cargo workspace (`crates/core`, `crates/platform`, `crates/service`, `app/src-tauri`) and npm workspaces (`app`, `extension`). *Acceptance:* `cargo build --workspace` and `npm install` succeed on Windows.
- [x] **T0.3** Scaffold Tauri 2 app in `app/` with React + TS + Vite. *Acceptance:* `npm run tauri dev -w app` opens a window showing "Monk Mode".
- [x] **T0.4** Add Tailwind + DaisyUI with `monk-light`/`monk-dark` themes, local fonts, keyframes (`breathe`, `fade-up`) and reduced-motion handling as in 6.1; add lucide-react. *Acceptance:* a `btn btn-primary` renders in `#C8643B`; contrast test passes.
  > NOTE: `primary-content` on `primary` in `monk-light` (`#FFFFFF` on `#C8643B`) is 3.9:1, below 4.5:1. It is a button label, not body text, so the test holds it to 3:1. Darken `primary` if button text should reach AA.
- [ ] **T0.5** Create the original logo SVG (6.1) and generate Tauri + extension icons. *Acceptance:* icon shows in window title bar, tray and extension toolbar.
  > NOTE: Logo and icons are generated (`npm run icons`) and the window title bar icon is verified. Left open because the tray arrives in T5.11 and the extension toolbar is checked when it is loaded in T4.1.
- [x] **T0.6** Lint/format: ESLint, Prettier, clippy, rustfmt; root scripts `lint`, `test`, `format`. *Acceptance:* `npm run lint` and `cargo clippy --workspace -- -D warnings` pass.
- [x] **T0.7** GitHub Actions CI on `windows-latest` (lint + tests + build). *Acceptance:* workflow runs green on push.
  > NOTE: Passed on the first push (run 41985972766). The owner then chose to run CI only in the final phase, so the workflow is manual-only (`workflow_dispatch`) until T7.2.

### Phase 1 — Core rules engine (`crates/core`, pure Rust, no OS calls)
- [x] **T1.1** Models: `Settings`, `Lock`, `UsageDay`, `FocusSession`, `StudyBlock`, `PomodoroState`, `State`; serde + ts-rs derives. Defaults match Section 4 (incl. TikTok domains in always-blocked). *Acceptance:* unit test checks all defaults.
- [x] **T1.2** Domain utils: clean/validate input (strip scheme, path, port, `www.`, lowercase), subdomain expansion (`""`, `www.`, `m.`, `mobile.`, `web.`), URL → domain match incl. subdomains. *Acceptance:* ≥ 15 table-driven tests incl. `https://www.YouTube.com/watch?v=1` → `youtube.com`.
- [x] **T1.3** Allowance logic: `tick(state, elapsed, now, active_tab)` counts only for a focused + visible allowance-site tab, not during a Pomodoro; returns events (`Warn2Min`, `Warn30s`, `AllowanceUsedUp`). *Acceptance:* tests for background tab, idle, non-allowance site, used up, multiple ticks, Pomodoro running.
- [x] **T1.4** Day rollover at local midnight; previous day saved to history. *Acceptance:* tests across midnight and DST change.
- [x] **T1.5** Pomodoro state machine: idle → focus → break → idle, counts focus time to the chosen study block. *Acceptance:* tests for full cycle, stop early, rollover during session.
- [x] **T1.6** `compute_blocked(state, now, extension_connected) -> BlockedSet { hosts_domains, url_patterns }` combining F1–F4 and F7. *Acceptance:* tests for: always-blocked always present; allowance sites present only when used up / Pomodoro / extension missing; Shorts patterns always present.
- [x] **T1.7** Lock validation: `validate_change(old, new, lock, now) -> Result<(), LooseningError>` implementing F5 allowed/forbidden lists. *Acceptance:* one test per allowed and per forbidden change.
- [x] **T1.8** Tamper-safe clock: `effective_now(now, last_seen) = max(now, last_seen)`. *Acceptance:* test clock rollback does not end lock or reset allowance.
- [x] **T1.9** Streak & stats (F8). *Acceptance:* tests for streak break, partial days, "Day X of 90".
- [x] **T1.10** Protocol message enums (5.2) + `npm run gen:types` (ts-rs export). *Acceptance:* generated `.ts` files compile in app and extension.

### Phase 2 — Windows platform layer (`crates/platform`)
- [x] **T2.1** Hosts file: write a managed block between `# >>> MONK MODE BLOCKER START` / `# <<< MONK MODE BLOCKER END`, both `0.0.0.0` and `::` lines, atomic write, keep other content and CRLF; path override via env `MONK_HOSTS_PATH` for tests; `ipconfig /flushdns` after change. *Acceptance:* tests on temp files: add, update, remove, foreign content preserved, idempotent.
- [x] **T2.2** Browser detection (read-only process list via `sysinfo`): is `chrome.exe` / `msedge.exe` / `brave.exe` running. **No process killing.** *Acceptance:* unit test with mocked process list.
  > NOTE: Returns which browsers are running, not just whether any are. F4 applies per browser, so Chrome with the extension must not excuse Edge without it.
- [x] **T2.3** Registry lock copy (`HKLM\SOFTWARE\MonkMode\LockEndsAt` + HMAC signature with a machine-specific key). *Acceptance:* round-trip test; tampered value rejected.

### Phase 3 — Background service (`crates/service`)
- [x] **T3.1** SQLite db + migrations (5.3) at `C:\ProgramData\MonkMode\monk.db` (override via `MONK_DATA_DIR`). *Acceptance:* migration test on temp dir.
- [x] **T3.2** Main loop: 1 s tick → core engine → enforce hosts (every 5 s or on change) → persist. *Acceptance:* integration test with fake platform layer.
- [x] **T3.3** WebSocket server `127.0.0.1:47613` for the extension; accepts only origin `chrome-extension://<fixed id>` + shared token created at install. *Acceptance:* test rejects wrong origin/token.
- [x] **T3.4** Named pipe server `\\.\pipe\monkmode` for the UI. *Acceptance:* round-trip `get_state` test.
- [x] **T3.5** Handlers for all message types; every settings change goes through `validate_change`. *Acceptance:* `update_settings` loosening while locked returns `LOCKED_LOOSENING`.
- [x] **T3.6** Push `state_changed` to all clients (throttled to 1/s). *Acceptance:* test client receives push after tick.
- [x] **T3.7** Extension-missing rule (F4): browser running + no heartbeat for 30 s → allowance sites blocked. *Acceptance:* integration test with fake clock.
- [ ] **T3.8** Watchdog: restart tray app in the user session if not running (`WTSQueryUserToken` / `CreateProcessAsUser`). *Acceptance:* manual test in `docs/manual-tests.md`; killed app is back in ≤ 10 s.
  > NOTE: Implemented, with steps in `docs/manual-tests.md` (T3, T4). Left open until run as SYSTEM in T7.3. Adds a `ui_quit` message so Quit in the tray (F9) is not undone by the watchdog; the flag clears when the UI connects again.
- [ ] **T3.9** Windows Service wrapper (`windows-service`): `install` / `uninstall` / `run` subcommands, auto-start, recovery = restart; refuse stop/uninstall while locked. *Acceptance:* manual test documented; `sc stop MonkModeService` while locked fails or the service comes back.
  > NOTE: Implemented, with steps in `docs/manual-tests.md` (T5, T6, T8, T12). Left open until run as SYSTEM in T7.3.
- [x] **T3.10** Logging with `tracing` to `C:\ProgramData\MonkMode\logs\` (rotating, 7 days). *Acceptance:* log file created on run.

### Phase 4 — Browser extension (`extension/`)
- [ ] **T4.1** MV3 scaffold (Vite + TS) with fixed `key` for a stable ID; permissions `tabs`, `storage`, `declarativeNetRequest`, `alarms`, `idle`, host permissions for listed sites. *Acceptance:* loads unpacked in Chrome and Edge without errors.
  > NOTE: Loads in Edge with the fixed ID, tested with the real service. Chrome no longer allows loading unpacked extensions from the command line, so Chrome stays for manual setup in T7.3. Host permission is `<all_urls>`, because users can add any site and redirect rules need host access to it.
- [x] **T4.2** WebSocket client: connect, `ext_hello` with token, heartbeat every 5 s, auto-reconnect. *Acceptance:* service logs `ext_hello` and heartbeats.
- [x] **T4.3** Active-tab tracker: sends `ext_active_tab` on tab switch, URL change, window focus change and every 5 s; uses `chrome.idle` (idle/locked = not counting). *Acceptance:* Vitest with mocked chrome APIs.
  > NOTE: A tab playing sound still counts when the user is idle, because `chrome.idle` only sees mouse and keyboard input and watching a video would otherwise be free. A muted video with no input for 2 minutes stops counting. A locked PC never counts.
- [x] **T4.4** Static Shorts/Reels `declarativeNetRequest` rules (F3) redirecting to the block page. *Acceptance:* manual: `/shorts/x` → block page, `/watch?v=x` plays.
- [x] **T4.5** Dynamic rules from service `BlockedSet.url_patterns` → redirect to block page (so blocked tabs already open also get redirected). *Acceptance:* Vitest for rule generation; manual: open YouTube tab is redirected when allowance hits 0.
- [ ] **T4.6** Content script hiding YouTube Shorts shelves/links. *Acceptance:* manual check on YouTube home; no console errors.
  > NOTE: The CSS is injected on YouTube in Edge. Left open until the shelves are checked by eye (`docs/manual-tests.md` B7), as selectors depend on YouTube's markup.
- [x] **T4.7** Block page (F10) styled with the same theme + breathing logo. *Acceptance:* shows progress & streak from service state.
  > NOTE: "Open my plan" sends a new `ext_open_app` message, and the service starts the app in the user session. The token reaches the extension as `token.json`, which the service writes to the installed extension folder.
- [x] **T4.8** `ext_blocked_attempt` reporting. *Acceptance:* blocked-attempt count increases in db.

### Phase 5 — Tauri app (`app/`)
- [x] **T5.1** Rust commands: named-pipe client exposing `get_state`, `update_settings`, `start_lock`, `start_pomodoro`, `stop_pomodoro`, `get_stats`; forward `state_changed` as a Tauri event. *Acceptance:* invoke from React returns state.
- [x] **T5.2** Zustand store + `useServiceState` hook; "Service not running" banner. *Acceptance:* Vitest for store.
- [x] **T5.3** Layout: sidebar `menu` with logo + 5 routes (Section 6), `fade-up` on route change. *Acceptance:* navigation works.
- [x] **T5.4** Today screen (F1, F7, F8 summary). *Acceptance:* Testing Library test with mock state.
- [x] **T5.5** Block lists screen (2 tabs) with add/remove and "move to always blocked"; loosening controls disabled with tooltip when locked. *Acceptance:* test that remove is disabled when locked.
- [x] **T5.6** Schedule screen: allowance minutes (max = current value when locked), Pomodoro lengths, study blocks, daily target. *Acceptance:* test.
- [x] **T5.7** Pomodoro UI: pick block, start/stop, countdown with breathing logo, notification at end. *Acceptance:* test with fake timers.
  > NOTE: The countdown shows `remaining_sec` from the service, which pushes it every second, so the UI has no timer of its own and the test drives it with state changes instead of fake timers.
- [x] **T5.8** Stats screen with SVG bars for 7/30 days. *Acceptance:* renders with mock data.
- [x] **T5.9** Lock screen + 3-step wizard with typed phrase. *Acceptance:* confirm button disabled until the phrase matches exactly.
- [x] **T5.10** First-run setup wizard (Section 6). *Acceptance:* shown only when `settings.setup_done = false`.
  > NOTE: `state_changed` and `get_state` also carry `connected_browsers` and `missing_browsers`, for "Waiting for extension…" and a notice on Today.
- [x] **T5.11** Tray icon + menu (F9), autostart, single instance, close-to-tray. *Acceptance:* manual test documented.
  > NOTE: Steps are in `docs/manual-tests.md` (A1-A4), to run in T7.3. Autostart is only registered in release builds. "Close to tray" closes the window for real and rebuilds it on open, because a hidden WebView2 held 166 MB. Start Pomodoro in the tray starts the first unfinished study block.
- [x] **T5.12** Notifications: 2 min / 30 s warnings, allowance used up, Pomodoro end. *Acceptance:* manual test documented.
  > NOTE: Steps are in `docs/manual-tests.md` (A5), to run in T7.3. The app works these out from state changes (tested in `notify.rs`).
- [x] **T5.13** About screen with honest limitations text (F6). *Acceptance:* present.

### Phase 6 — Installer & packaging
- [x] **T6.1** Tauri NSIS bundle (per-machine, admin), includes `monk-service.exe`. *Acceptance:* `npm run tauri build -w app` produces an installer in CI.
  > NOTE: Files are copied by `installer/hooks.nsh` rather than Tauri resources, so a plain `cargo build` does not need release builds of the service and extension to exist.
- [ ] **T6.2** Install hook: install + start service, create extension token, create `C:\ProgramData\MonkMode` with ACL (SYSTEM + Admins write, Users read), apply TikTok block immediately. *Acceptance:* fresh Windows install → service running, tiktok.com blocked.
  > NOTE: Implemented in the hooks and `monk-service install`. Left open until an elevated install is run in T7.3.
- [ ] **T6.3** Uninstall hook: asks service; aborts with a message while locked; otherwise stops service, removes hosts block, deletes registry key. *Acceptance:* manual test both cases.
  > NOTE: Implemented in the hooks and `monk-service uninstall`. Left open until run in T7.3. Upgrades are also refused during a lock, because Tauri upgrades run the old uninstaller.
- [ ] **T6.4** Extension packaged as `.zip` CI artifact + README steps for loading it unpacked. *Acceptance:* artifact in CI.

### Phase 7 — Verification
- [x] **T7.1** Write `docs/manual-tests.md` covering every item in Section 8.
- [ ] **T7.2** All automated tests + lint green on Windows CI.
  > NOTE: Restore the `push` and `pull_request` triggers in `.github/workflows/ci.yml` first; they were removed during development (see T0.7). Until then, phases are checked locally with `npm run lint`, `npm run test`, `cargo clippy` and `cargo test --workspace`.
- [ ] **T7.3** Run the manual tests on the owner's Windows PC (or a Windows VM) and record results. *Acceptance:* all Section 8 criteria pass, or failures are added as new tasks.
- [x] **T7.4** Measure idle CPU/RAM. *Acceptance:* within Section 8 limits.
  > NOTE: Measured with release builds, idle in the tray: 0.05% CPU and 26 MB working set (service + UI), against limits of 1% and 120 MB. Closing the window destroys it to free WebView2. With the window open, WebView2 brings it to about 150 MB private memory (376 MB working set, which double-counts shared pages), over the limit. That is active use rather than idle, so this is recorded here and not as a failure.
- [x] **T7.5** README: what it is, install, extension setup, how the lock works, limitations, screenshots.

---

## 10. Risks & decisions

| Risk | Decision |
|---|---|
| Hosts file is domain-level only | Extension handles URL-level blocking (Shorts/Reels); F4 blocks allowance sites if the extension is missing |
| Browser "Secure DNS" (DNS-over-HTTPS) may skip the hosts file | Check in T7.3. If it fails, the installer sets Chrome/Edge/Brave policy `DnsOverHttpsMode=off` in the registry (add as new task) |
| User can run as admin and kill things | Service runs as SYSTEM + recovery restart + registry lock copy; honest limitation documented |
| Antivirus flags hosts-file edits | Document in README; code signing in Backlog |
| Extension ID changes between unpacked installs | Fixed `key` in manifest |

---

## 11. Backlog (not v1)

- Publish extension to Chrome Web Store
- Code signing
- Desktop app blocker (kill `.exe` like Spotify/Discord), if ever wanted
- Accountability partner: weekly report
- Forward clock tampering: moving the system clock ahead ends a lock early and refills the allowance. F5 only covers rollback; a fix needs a time source the user cannot edit (for example, network time)

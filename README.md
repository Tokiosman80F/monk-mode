# Monk Mode Blocker

A free Windows app for staying off distracting sites during long study stretches.

- **Daily limits per site:** YouTube gets 30 minutes and other sites 10 by default (you choose 0–180 for each), counted only while you're actually watching. Each site closes until midnight when its time runs out.
- **Always blocked:** the worst sites, like TikTok, stay closed.
- **Shorts and Reels blocked:** YouTube Shorts, Instagram Reels and Facebook Watch are closed even with time left. Normal videos still play.
- **Commitment lock:** pick an end date, and until then your rules can only get stricter.
- **Today's tasks:** add and remove tasks with subtasks and a priority, a Pomodoro timer, a streak and simple charts.
- **XP and levels:** earn XP for focus time and finished tasks, level up and unlock badges.

Desktop apps like Spotify and Discord are never blocked or closed.

![The Today screen](docs/screenshots/today.png)

![The block page](docs/screenshots/block-page.png)

## Install

Windows 10 or 11 (x64), with Chrome, Edge or Brave.

1. Download the installer from the **installer** artifact of a [CI run](https://github.com/Tokiosman80F/monk-mode/actions), or [build it yourself](#develop).
2. Run it and approve the admin prompt. This installs the background service, and `tiktok.com` is blocked from then on.
3. The app opens a setup wizard for the remaining steps.

### Add the browser extension

Do this in **every** browser you use. A browser running without the extension gets its sites with limits blocked after 30 seconds, because without it the app can't measure your time.

1. Open `chrome://extensions` (`edge://extensions` in Edge, `brave://extensions` in Brave).
2. Turn on **Developer mode**.
3. Choose **Load unpacked**, and pick `C:\Program Files\Monk Mode\extension`. The wizard has a button that opens this folder.
4. In the extension's details, turn on **Allow in Incognito**.

Use the installed folder, not the **extension** CI artifact. The installed copy holds the token that lets it talk to the service.

## How the lock works

While a lock is active:

| You can                       | You can't                             |
| ----------------------------- | ------------------------------------- |
| Add sites to either list      | Remove any site                       |
| Move a site to always blocked | Give an always blocked site a limit   |
| Lower a site's daily limit    | Raise a site's daily limit            |
|                               | Turn off the Shorts and Reels blocker |
|                               | Shorten or remove the lock            |
|                               | Stop the service or uninstall the app |

The rules are enforced by a Windows Service, not the app, so closing the app changes nothing. The service rewrites its hosts file block every 5 seconds. It keeps the lock's end date in its database and as a signed copy in the registry, so deleting one doesn't unlock. Turning the clock back doesn't shorten a lock either.

Starting a lock asks you to type `I commit to monk mode`. **There's no undo.** Try a short one first.

## Limitations

The goal is to make giving up slow and annoying, not impossible. An admin user can still get around it:

- **Safe Mode or a Windows reinstall** gets past the service.
- **Moving the system clock ahead** ends a lock early and refills the limits. Only moving it back is protected.
- **Other browsers**, such as Firefox, aren't monitored. The hosts file still blocks sites once their limits are used up, but time spent there isn't counted.
- **Secure DNS (DNS over HTTPS)** can skip the hosts file in a browser. The extension still blocks the site there.
- **A muted video** with no mouse or keyboard input stops counting after 2 minutes, because the browser reports you as idle.
- **Upgrades** can't be installed during a lock, because upgrading runs the old uninstaller.
- **Antivirus tools** may flag the app, since it edits the hosts file and isn't code-signed yet. Windows Defender flagged the installer as `Trojan:Win32/Bearfoos.B!ml` during testing, a machine-learning detection. Only install a copy you built or got from this repo's CI.

## How it's built

```
crates/core       Rules engine: limits, lock rules, tasks, XP. Pure Rust, no OS calls.
crates/platform   Windows: hosts file, browser detection, registry, user-session launch.
crates/service    monk-service.exe: the main loop, SQLite, WebSocket and named pipe servers.
app/              Tauri 2 app: React, TypeScript and DaisyUI, plus the tray.
extension/        Manifest V3 extension: active-tab timer, URL blocking, block page.
installer/        NSIS hooks that install the service and extension.
docs/             Manual tests.
```

The service is the only source of truth. The app and extension send it requests, and it validates every one. See [PRD.md](PRD.md) for the full design.

## Develop

You need Node 20+, Rust with the MSVC toolchain, and the Visual Studio C++ Build Tools.

```bash
npm install
```

```bash
npm run build -w app
```

`cargo` needs `app/dist` to exist, so run that build once first.

To try the service without touching the real hosts file or data, set `MONK_HOSTS_PATH` and `MONK_DATA_DIR` to temp paths, then:

```bash
cargo run -p monk-service -- console
```

In another terminal, start the app:

```bash
npm run tauri dev -w app
```

Before committing:

```bash
npm run lint
```

```bash
npm run test
```

```bash
cargo clippy --workspace --all-targets -- -D warnings
```

```bash
cargo test --workspace
```

Build the installer with `npm run tauri build -w app`. It ends up in `target/release/bundle/nsis/`. After changing a message or model in `crates/core`, run `npm run gen:types`.

Automated tests can't cover a real service install or real browsers, so [docs/manual-tests.md](docs/manual-tests.md) lists those checks.

## License

[MIT](LICENSE)

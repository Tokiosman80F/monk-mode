# Monk Mode Blocker

Free Windows desktop app: a small daily allowance for distracting sites, permanent blocks for the worst ones, and a commitment lock that can't be weakened before its end date. See [PRD.md](PRD.md).

Status: in development.

## Develop

Needs Node 20+, Rust (MSVC toolchain) and Visual Studio C++ Build Tools.

    npm install
    npm run build -w app   # cargo needs app/dist to exist
    npm run tauri dev -w app
    npm run lint
    npm run test
    cargo test --workspace

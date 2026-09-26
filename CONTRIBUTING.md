# Contributing to VRChat Asset Manager

Thanks for your interest in contributing! This guide will walk you through
everything you need to get the project running on your own computer.

---

## What this app is built with

You do not need to know all of these in depth to contribute, but it helps to
know they exist:

| Layer | Technology | What it does |
|---|---|---|
| Desktop shell | [Tauri v2](https://v2.tauri.app/) | Wraps the app as a native desktop program |
| Backend logic | [Rust](https://www.rust-lang.org/) | File operations, database, scraping |
| Frontend UI | [React](https://react.dev/) + [TypeScript](https://www.typescriptlang.org/) | Everything you see on screen |
| Build tool | [Vite](https://vitejs.dev/) | Fast local development server |
| Database | SQLite (via `rusqlite`) | Stores all your library metadata |

---

## Setting up your development environment

### Step 1 — Install Node.js

Node.js runs the JavaScript build tools.

1. Go to https://nodejs.org
2. Download the **LTS** version (the one labeled "Recommended For Most Users")
3. Run the installer and follow the prompts

To check it worked, open a terminal and run:
```
node --version
```
You should see a version number like `v20.x.x`.

### Step 2 — Install Rust

Rust compiles the backend of the app.

1. Go to https://rustup.rs
2. Follow the instructions for your operating system
3. Restart your terminal after installation

To check it worked:
```
rustc --version
```

### Step 3 — Install Tauri's system dependencies

Tauri needs some extra tools depending on your operating system.

Follow the official guide here:
👉 https://v2.tauri.app/start/prerequisites/

This is the most platform-specific step. The guide covers Windows, macOS, and Linux.

### Step 4 — Clone the repository and install packages

```bash
git clone https://github.com/YOUR_USERNAME/VRChatAssetManager.git
cd VRChatAssetManager
npm install
```

### Step 5 — Start the app in development mode

```bash
npm run tauri dev
```

The first time you run this, Rust will compile the backend — this takes a few
minutes. Subsequent runs are much faster.

---

## Building a release version

To create an installable version of the app:

```bash
npm run tauri build
```

The output (an installer or executable) will appear in
`src-tauri/target/release/bundle/`.

---

## Project structure (quick reference)

```
VRChatAssetManager/
├── src/                      # Frontend (React + TypeScript)
│   ├── components/           # UI components (buttons, modals, cards, etc.)
│   ├── api.ts                # All calls to the Rust backend go through here
│   ├── library.ts            # Helper functions for reading library data
│   ├── types.ts              # TypeScript type definitions
│   └── main.tsx              # App entry point
│
├── src-tauri/src/            # Backend (Rust)
│   ├── db.rs                 # Database schema and queries
│   ├── items.rs              # Model add/edit/remove operations
│   ├── assets.rs             # Asset add/edit/remove operations
│   ├── filing.rs             # Moving files into the library structure
│   ├── images.rs             # Image import and management
│   ├── reference.rs          # Sites, creators, categories
│   ├── scraper.rs            # Page scraping (Booth, Gumroad, Jinxxy, Payhip)
│   ├── sync.rs               # Reconcile metadata with files on disk
│   └── commands.rs           # Tauri command entry points (called by frontend)
│
├── LICENSE                   # MIT License
├── AI_DISCLOSURE.md          # How AI was used in this project
└── CONTRIBUTING.md           # This file
```

---

## How the frontend and backend talk to each other

The frontend calls Rust functions using Tauri's `invoke()` system.
All of those calls are wrapped in `src/api.ts` — if you want to add a new
backend feature, you'll add a function there.

The Rust side exposes functions with `#[tauri::command]` in `commands.rs`
and the other `src-tauri/src/*.rs` files.

---

## Coding conventions

- **TypeScript**: use the types defined in `src/types.ts`. Do not use `any`.
- **Rust**: run `cargo fmt` before committing. The project follows standard
  Rust idioms.
- **Comments**: explain *why* something works the way it does, not *what* it
  does (the code should make the "what" obvious).

---

## Submitting changes

1. Fork the repository on GitHub
2. Create a new branch: `git checkout -b my-feature-name`
3. Make your changes
4. Test that `npm run tauri dev` still works
5. Commit with a clear message explaining what changed and why
6. Open a Pull Request on GitHub

---

## Getting help

If you're stuck on setup, open a GitHub Issue and describe what step you're
on and what error message you see.

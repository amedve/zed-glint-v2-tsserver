# Glint v2 for Zed — `.gts` / `.gjs` editor support

Adds working Go to Definition, Find References, hover, rename, and Glint diagnostics inside Ember `.gts` and `.gjs` files in [Zed](https://zed.dev) — including jumping from a `<MyComponent />` tag in a template to the component's definition.

> This extension isn't on the Zed extension registry yet — install it as a **dev extension** (instructions below).

## Requirements

- **The published [Ember extension by James Lamont](https://github.com/jylamont/zed-ember) must be installed in Zed.** This extension piggybacks on it for syntax highlighting and the `Glimmer (TypeScript)` / `Glimmer (JavaScript)` language registrations — without it, nothing works.
- A project set up for **Glint v2** — `@glint/tsserver-plugin` must be in your project's dependencies. Verify with `ls node_modules/@glint/`; you should see at least `tsserver-plugin`.
- **`typescript-language-server`** reachable from your project. Project-local is recommended so you get the project's TS version:

  ```sh
  npm install -D typescript-language-server typescript
  ```

  Or install it globally if you'd rather not add more dependencies to your project(s):

  ```sh
  npm install -g typescript-language-server typescript
  ```

  The extension looks for a project-local install first, then falls back to whatever's on `PATH` (`vtsls` works too).
- **Rust toolchain with the `wasm32-wasip1` target** — Zed compiles the extension's Rust source to WASM on your machine when you install it. One-time install:

  ```sh
  # Cross-platform, via the official installer:
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
  ```

  Then add the WASM target (reload or open new shell):

  ```sh
  rustup target add wasm32-wasip1
  ```

## Setup

### 1. Install the Ember extension

In Zed, `cmd-shift-x`, search for **Ember** (by James Lamont), Install.

### 2. Install this extension as a dev extension

Clone or copy this repo to Zed dev extensions folder (`~/.config/zed/dev-extensions/zed-glint-v2-tsserver`). Then in Zed's Extensions view (`cmd-shift-x`), click **Install Dev Extension** in the top-right and point it at the folder. The first build takes 30–60 seconds.

### 3. Wire up `settings.json`

Open Zed's settings (`cmd-,`) and merge in:

```json
{
  "file_types": {
    "Glimmer (JavaScript)": ["gjs"],
    "Glimmer (TypeScript)": ["gts"]
  },
  "languages": {
    "Glimmer (TypeScript)": {
      "language_servers": ["glint", "..."]
    },
    "Glimmer (JavaScript)": {
      "language_servers": ["glint", "..."]
    }
  }
}
```

The parentheses in `"Glimmer (TypeScript)"` are literal — those are the names the Ember extension registers.

### 4. Restart Zed and open a `.gts` file

The status bar should show **Glimmer (TypeScript)**. Clicking it should list **Glint via tsserver** as active. `f12` on a component tag should jump to its definition.

## Verifying Glint is actually loaded

- `f12` on a component tag should jump to its definition.
- Hover over a `{{this.foo}}` expression inside a `<template>` block. If Glint is loaded you get a type signature; if not, nothing.

## Troubleshooting

**"Failed to compile Rust extension"** when installing. The `wasm32-wasip1` target isn't installed. Run `rustup target add wasm32-wasip1`, then rebuild via Extensions → the entry for this extension → Rebuild.

**"No TypeScript language server found".** Install `typescript-language-server` project-locally (`npm install -D typescript-language-server typescript`) or globally, then reload Zed.

> macOS: if you launch Zed from Spotlight/Finder, its `PATH` may not include `nvm`/`asdf`/Homebrew binaries. Run `launchctl setenv PATH "$PATH"` once from your shell.

**Status bar shows plain "TypeScript" instead of "Glimmer (TypeScript)".** The Ember extension isn't installed or enabled, or your `file_types` block doesn't use the exact names `"Glimmer (TypeScript)"` / `"Glimmer (JavaScript)"` (with parentheses).

**`f12` works but Glint-specific features don't (no template hover, no template diagnostics).** The plugin didn't load. Open `cmd-shift-p → dev: open language server logs → Glint via tsserver` and look for an `@glint/tsserver-plugin` line — or a complaint that it couldn't be found.

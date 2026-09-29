# Nebula Launcher

A fast, simple Minecraft launcher built with **Rust + Tauri 2**, **Svelte 5** (runes), **Vite** and **Tailwind CSS v4**.

## Features

- **Every official version** from Mojang's manifest: releases, snapshots, **old betas** and **old alphas**.
- Downloads the **real Mojang `client.jar`**, libraries and assets, and verifies every file with SHA-1.
  Legacy asset layouts (`pre-1.6`, `legacy`) and native library extraction are handled for the old versions.
- **Offline accounts** with vanilla-compatible UUIDs (`OfflinePlayer:<name>`).
- **Ely.by accounts** (Yggdrasil API `authserver.ely.by`) including two-factor auth, automatic token
  validate/refresh, skin heads in the UI and in-game skins through **authlib-injector** (downloaded automatically).
- **Instances**: each with its own game folder, version, memory, JVM args, Java path, window size, fullscreen and
  optional auto-join server. Duplicate, edit, delete (optionally with files).
- **Automatic Java**: downloads the right official Mojang runtime (Java 8 for old versions up to the newest) —
  or use your own Java path. Includes a "Test Java" button.
- Live **console** with log colouring, copy/clear/kill; playtime and last-played tracking.
- Parallel downloads with live progress, offline-cached version list, "hide launcher while playing" option.

## Development

Requirements: Node 20+, Rust (stable) and the [Tauri 2 prerequisites](https://tauri.app/start/prerequisites/).

```bash
npm install
npm run tauri dev      # desktop app
npm run dev            # UI only, in a browser, with a mock backend
npm run tauri build    # installers / bundles
```

Running `npm run dev` in a normal browser uses `src/lib/mock.ts` as a fake backend so the UI can be developed
without Rust. Inside Tauri the real commands in `src-tauri/src/commands.rs` are used.

## Layout

```
src/                     Svelte 5 frontend (pages, components, store)
src-tauri/src/
  versions.rs            Mojang manifest + version JSON models, rule evaluation
  install.rs             client jar / libraries / assets / natives
  java.rs                Mojang Java runtime download
  auth.rs                offline UUIDs, Ely.by login, authlib-injector
  launch.rs              argument building, process supervision, log streaming
  commands.rs            Tauri commands
```

Data (shared game files, Java runtimes, instances, `launcher.json`) is stored in the OS app-data directory
(`dev.nusaxd.nebula-launcher`). Use *Settings → Storage → Open* to find it.

## Notes

- Microsoft/Xbox login is intentionally not included (it requires a registered Azure application ID).
- Offline / Ely.by accounts cannot join Mojang-authenticated ("online-mode") servers.

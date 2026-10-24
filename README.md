# Excalidraw Portfolio Manager

A local-first desktop portfolio manager built with [Tauri 2](https://tauri.app/), Rust, React, and the Excalidraw drawing engine.

The application organizes drawings into persistent portfolios and boards while keeping all data on the local machine. It does not require accounts, a server, or collaboration services.

## Features

- Multiple portfolios and boards
- Independent scene persistence for every board
- Atomic local JSON storage
- Native desktop menus for portfolio and board navigation
- Light and dark themes
- Excalidraw drawing tools and variable-width freehand strokes
- Linux-native stylus pressure bridge for the Xenx X1-640

## Architecture

- `excalidraw-app/` contains the desktop application UI.
- `packages/excalidraw/` contains the drawing engine and its desktop-specific pressure handling.
- `src-tauri/` contains the Rust application, native menus, persistence, and Linux input integration.

Portfolio data is stored in the Tauri application data directory as `portfolios.json`. Drawing scenes are stored inside their corresponding board records.

## Requirements

- Node.js 18 or newer
- Yarn 1.22
- Rust 1.90 or newer
- Tauri 2 system dependencies
- GTK and WebKitGTK development packages on Linux

For Linux stylus pressure, the tablet must expose a readable HID device. The current native parser targets the Xenx X1-640 protocol and is intentionally marked experimental.

## Getting started

Install JavaScript dependencies:

```bash
yarn install
```

Start the Tauri development application:

```bash
npx @tauri-apps/cli@2.12.4 dev
```

Build the frontend:

```bash
yarn build
```

Run the type checker:

```bash
yarn test:typecheck
```

## Data and privacy

All portfolio and board data is stored locally by the Rust backend. No account, cloud database, or collaboration server is configured in this application.

## Project status

This is an actively developed desktop-focused fork of Excalidraw. The portfolio manager and persistence layers are functional. Linux stylus pressure support is hardware-specific and remains under validation.

## License

This project is released under the MIT License. It is built on the open-source [Excalidraw](https://github.com/excalidraw/excalidraw) project.

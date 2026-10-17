# Tauri 2 with a Vue 3 + TypeScript UI and a Rust core

The app has to scan entire drives quickly and stay lightweight, since it is a "PC optimizer". We chose Tauri 2: scanning, file Operations and Windows API access live in a Rust core, and the UI is built with Vue 3 + TypeScript, leveraging the author's TS experience. Perceived performance depends on the core and on list virtualization, not on the UI framework.

## Considered Options

- **C# + WPF/.NET**: similar performance and great Windows access, but XAML would be new to the author and the app would be larger.
- **Electron**: familiar (Node), but at ~150 MB it contradicts the premise of an app that optimizes your PC.
- **Python + Qt**: slow scanning and poor distribution.
- **React instead of Vue**: equivalent performance; Vue was chosen by author preference.

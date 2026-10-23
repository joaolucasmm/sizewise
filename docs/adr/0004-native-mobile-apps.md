# Mobile apps are native (SwiftUI and Jetpack Compose), not Tauri

The desktop app uses Tauri (ADR 0001), so a reader would expect the mobile apps to reuse it. We chose native apps instead: SwiftUI on iOS and Kotlin + Jetpack Compose on Android, living in the same repository and sharing the glossary, ADRs and issue tracker, but no code. On mobile the Rust `scanner` has little to offer: iOS does not allow walking the file system, and on Android the right source is the system media index, not a folder walk. What mobile does need — smooth grids of thousands of photo thumbnails, home-screen widgets, background work, mature tooling — is where native frameworks are strong and Tauri mobile is still young. Permissions are the same either way; the OS, not the framework, decides what an app can access.

## Considered Options

- **Tauri 2 mobile**: one UI codebase and a "write once" story, but a WebView photo grid, no widgets and a young mobile toolchain.

## Consequences

- Android ships first (author's own device); iOS follows using a borrowed Mac and iPhone.
- Mobile features center on freeing space (media, Duplicates, unused apps); no "speed up" claims, in line with store rules.

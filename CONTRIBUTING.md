# Contributing to WhatBubbles

WhatBubbles is a C++/ImGui rewrite of the OpenBubbles Windows client. The protocol core stays in Rust (`rust/` + `rustpush/`); the frontend is native C++.

## Toolchain

You need:

- [Git](https://git-scm.com/downloads)
- [Visual Studio 2022](https://visualstudio.microsoft.com/downloads/) with the **Desktop development with C++** workload (includes MSVC, the Windows SDK, and the Windows 10/11 platform toolset)
- [CMake](https://cmake.org/download/) 3.24 or newer (3.24 is the minimum for the Corrosion integration used in `CMakeLists.txt`)
- [Rust](https://rustup.rs/) (stable, default MSVC toolchain on Windows)
- [Protocol Buffers compiler (`protoc`)](https://github.com/protocolbuffers/protobuf/releases) on `PATH` — needed by `prost-build` when the Rust core compiles
- Strawberry Perl or another Perl implementation on `PATH` — needed by `openssl-sys` (the Rust protocol core vendors OpenSSL and requires Perl at build time)

Optional but recommended:

- [Ninja](https://ninja-build.org/) — CMake picks it up automatically when present and builds faster than the default MSBuild generator
- [vcpkg](https://github.com/microsoft/vcpkg) if you prefer to manage third-party C++ dependencies outside of CMake `FetchContent`

## Submodules

After cloning, initialize the `rustpush` submodule (which itself has nested submodules):

```
git submodule update --init --recursive
```

If any of the nested submodules are declared with SSH URLs and you don't have SSH keys configured for GitHub, run once:

```
GIT_CONFIG_COUNT=1 GIT_CONFIG_KEY_0=url.https://github.com/.insteadOf GIT_CONFIG_VALUE_0=git@github.com: git submodule update --init --recursive
```

## Building

There is a `CMakePresets.json` at the repo root that points CMake at the rustup-managed toolchain directly (required because Corrosion's rustup proxy detection misbehaves in MSYS / Git Bash environments on Windows).

Visual Studio 2022 generator:

```
cmake --preset win-vs
cmake --build --preset win-vs-release
```

Ninja (run from a VS 2022 Developer Command Prompt so `cl.exe`, `link.exe`, and the Windows SDK are on `PATH`):

```
cmake --preset win-ninja
cmake --build --preset win-ninja
```

Cargo is invoked by Corrosion during the CMake build; you do not need to run `cargo build` yourself. To iterate on the Rust side in isolation:

```
cargo check --manifest-path rust/Cargo.toml
```

First build is slow — the Rust core vendors OpenSSL and pulls the full `rustpush` tree.

## Style

- **C++:** C++17. Default to standard library containers and algorithms. Avoid hand-rolled smart pointers; prefer `std::unique_ptr` / `std::shared_ptr`. Match Dear ImGui's immediate-mode idioms in UI code (no retained widget trees).
- **Rust:** idiomatic, `rustfmt` on save, `clippy::pedantic` as a warning guide (not a hard gate). FFI signatures live in `rust/src/ffi.rs` under the `cxx::bridge` module.
- **No AI trailers, artifacts, or attribution** in source, comments, commit messages, docs, or license files.

## Branches and commits

1. Create a topic branch: `git checkout -b <short-topic>`.
2. Keep commits focused. Commit messages describe the *why*.
3. Rust changes that touch the FFI boundary should come with a matching C++ caller update in the same commit so the tree always builds.

## Pull requests

Open PRs against the default branch. Include:

- What problem is being solved.
- Summary of the change.
- Manual test notes for the Windows build.

## Upstream

WhatBubbles is an independent fork. Do not send WhatBubbles changes upstream to OpenBubbles/BlueBubbles, and do not file WhatBubbles bugs on upstream issue trackers.

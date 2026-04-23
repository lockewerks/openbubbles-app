# WhatBubbles

WhatBubbles is an open-source, desktop-first iMessage client for Windows. It is a hard fork of [OpenBubbles](https://github.com/OpenBubbles/openbubbles-app), rewritten as a native C++ application with [Dear ImGui](https://github.com/ocornut/imgui), reusing the original Rust protocol core via an `cxx` FFI bridge.

WhatBubbles is a client alternative for the OpenBubbles ecosystem. It is not a replacement for the OpenBubbles service, relay, or any backend infrastructure. WhatBubbles requires access to a Mac and an Apple ID, along with a compatible OpenBubbles-flavoured service, to function.

## Status

Early development. The Rust protocol core (`rust/` + `rustpush/`) is inherited from OpenBubbles. The C++/ImGui frontend is being built from scratch. Do not expect feature parity with upstream yet.

Only the Windows desktop build is in scope.

## Stack

- **Frontend:** C++17, Dear ImGui, Win32 + Direct3D 11 backend.
- **Protocol core:** Rust, reusing the `rustpush` crate (Apple ID auth, APNs push, IDS lookup, iMessage send/receive).
- **FFI:** the [`cxx`](https://cxx.rs) crate, surfaced in `rust/src/ffi.rs`.
- **Build system:** CMake, integrating Cargo via [Corrosion](https://github.com/corrosion-rs/corrosion).

## Layout

```
WhatBubbles/
├── app/            # C++/ImGui frontend
├── rust/           # Rust FFI bridge (cxx) + shared helpers
├── rustpush/       # Protocol core (submodule, inherited from OpenBubbles)
├── CMakeLists.txt  # Top-level build
├── LICENSE
├── NOTICE
├── README.md
├── CONTRIBUTING.md
└── CODE_OF_CONDUCT.md
```

## Building

See `CONTRIBUTING.md` for the full toolchain setup and commands.

## Acknowledgments

WhatBubbles is a fork of [OpenBubbles](https://github.com/OpenBubbles/openbubbles-app), which itself builds on [BlueBubbles](https://github.com/BlueBubblesApp/bluebubbles-app). Both projects are licensed under the Apache License, Version 2.0; the same license applies to this fork.

The Rust protocol core under `rust/` (heavily rewritten for this port) and `rustpush/` (unchanged) originate from those projects. Substantial credit to their contributors for the reverse-engineering work behind the iMessage protocol support.

The "Bubbles" name is used only to describe the origin of the work, as permitted by Section 6 of the Apache License, Version 2.0. WhatBubbles is not endorsed by, affiliated with, or sponsored by either upstream project.

## License

Apache License, Version 2.0. See [`LICENSE`](./LICENSE) for the full text and [`NOTICE`](./NOTICE) for attribution details.

# meme_cutscene_skip

A standalone mod for **Metal Gear Rising: Revengeance** — in-engine cutscene skip
"like on consoles" (for the `P370_RESTART` / `P370_IN` scenes → `P370_EVENT`).

## Components

- **`meme_cutscene_skip.exe`** — launcher (released as
  `meme_cutscene_skip_injector.zip`): finds the game (next to itself → known
  Steam path → `--exe`), starts it when needed, injects the embedded DLL and can
  optionally tail the mod log.
- **`meme_cutscene_skip_lib.dll`** — the mod: MinHook on
  `cSlowRateManager::updateFrameTime` (`0xA03970`, once per main-loop iteration)
  plus the cutscene-skip frame state machine. It is the main download, released as
  `meme_cutscene_skip.zip`: the same DLL named `meme_cutscene_skip_lib.asi` and
  nested under `plugins/`, for an external ASI loader (see below).

The DLL is embedded into the launcher (`include_bytes!`) and extracted to
`%LOCALAPPDATA%\meme_cutscene_skip\` at launch.

**Logging is debug-only.** Debug builds append
`%LOCALAPPDATA%\meme_cutscene_skip\meme_cutscene_skip.log`; release builds
(`LOG_ENABLED = cfg!(debug_assertions)`) write nothing, so end users get no stray
file. `--follow` therefore only makes sense in a debug build.

## Building

```sh
cargo build --release
```

`build.ps1` builds the release and packages the archives — CI runs this same script:

```powershell
pwsh -File build.ps1
```

→ `out/meme_cutscene_skip.zip` (the mod as `plugins/meme_cutscene_skip_lib.asi`)
and `out/meme_cutscene_skip_injector.zip` (the launcher).

The output is `target/i686-pc-windows-msvc/release/meme_cutscene_skip.exe` (+ the
DLL next to it) — the DLL is embedded into the launcher, so a single exe is enough
to ship. The crate builds for `i686-pc-windows-msvc` (the game is 32-bit), as set
in `.cargo/config.toml`; MSVC C++ build tools are required (MinHook's C code is
compiled).

Tests (a fresh debug build is required: the launcher embeds the DLL via
`include_bytes!`, and without it `cargo test` cannot be built):

```sh
cargo build                       # the debug build creates the DLL payload
cargo test
cargo test --lib                  # state-machine logic only, no DLL build
```

## CI and releases

On a `v*` tag push, `.github/workflows/build.yml` runs `build.ps1` on the Windows
runner and attaches the two archives (`out/meme_cutscene_skip.zip`, the mod as an
`.asi` plus `readme.txt` with the install steps for end users, and
`out/meme_cutscene_skip_injector.zip`, the self-injecting launcher) to a GitHub
Release. No other publishing step: the workflow only needs the default
`GITHUB_TOKEN`.

## Running

```sh
cargo run                 # find/start the game and inject
cargo run -- --follow     # same, plus print the mod log to the console
cargo run -- --kill-first --follow   # clean game start (for a fresh DLL)
```

Flags: `--exe <path>`, `--kill-first`, `--no-launch`, `--follow` (debug builds),
`--timeout <sec>`. If the game is already running, the launcher injects into it.

**Live development loop.** `LoadLibraryW` does not call `DllMain` a second time, so
a freshly built DLL is not picked up by an already-loaded process — restart the game
for new code (`--kill-first`). We do not `FreeLibrary`/unload: hot DLL reloading
crashed the game in drmod-rs.

## Using an external ASI loader instead

`meme_cutscene_skip.zip` is the mod as an `.asi`: the same DLL, renamed
`meme_cutscene_skip_lib.asi` and nested under `plugins/`, plus a `readme.txt`
carrying the steps below for end users (the text lives in
[`asi-readme.txt`](asi-readme.txt) and is copied in by `build.ps1`). No loader yet?
Download the latest **Win32** `d3d9.dll` from
[Ultimate-ASI-Loader releases](https://github.com/ThirteenAG/Ultimate-ASI-Loader/releases).

Unpack both into the game's root, so the tree reads:

```
Metal Gear Rising REVENGEANCE
|-- METAL GEAR RISING REVENGEANCE.EXE
|-- d3d9.dll
|-- /plugins
     |-- meme_cutscene_skip_lib.asi
```

`d3d9.dll` loads the plugin when the game starts.

The alternative is `meme_cutscene_skip_injector.zip`: the launcher, which starts the
game itself and injects the embedded DLL — no loader needed, but a second process.

## Dependencies

- `windows` — Win32 API (memory, threads, ToolHelp, windows).
- MinHook — vendored in `vendor/minhook` (BSD-2-Clause, `LICENSE.txt`) to keep the
  repository self-contained.

## License

MIT — see [LICENSE](LICENSE).

The vendored MinHook under `vendor/minhook` is licensed separately under
BSD-2-Clause — see `vendor/minhook/LICENSE.txt`.

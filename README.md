# meme_cutscene_skip

A standalone mod for **Metal Gear Rising: Revengeance** — in-engine cutscene skip
"like on consoles" (for the `P370_RESTART` / `P370_IN` scenes → `P370_EVENT`).

## Components

- **`meme_cutscene_skip.exe`** — launcher: finds the game (next to itself → known
  Steam path → `--exe`), starts it when needed, injects the embedded DLL and can
  optionally tail the mod log.
- **`meme_cutscene_skip_lib.dll`** — the mod: MinHook on
  `cSlowRateManager::updateFrameTime` (`0xA03970`, once per main-loop iteration)
  plus the cutscene-skip frame state machine.

The DLL is embedded into the launcher (`include_bytes!`) and extracted to
`%LOCALAPPDATA%\meme_cutscene_skip\` at launch. The log lives there as well:
`%LOCALAPPDATA%\meme_cutscene_skip\meme_cutscene_skip.log`.

## Building

```sh
cargo build --release
```

The output is `target/i686-pc-windows-msvc/release/meme_cutscene_skip.exe` (+ the
DLL next to it) — the DLL is embedded into the launcher, so a single exe is enough
to ship. The crate builds for `i686-pc-windows-msvc` (the game is 32-bit), as set
in `.cargo/config.toml`; MSVC C++ build tools are required (MinHook's C code is
compiled).

For an even smaller binary there is UPX — see `build.ps1`:

```powershell
pwsh build.ps1           # cargo build --release + upx --best --lzma exe
pwsh build.ps1 -NoUpx
```

⚠️ UPX packs **the launcher only** (exe ~523 KB → ~192 KB; the DLL is inside). The
DLL in `target/` must not be packed: it is the payload embedded into the exe at
compile time and extracted to `%LOCALAPPDATA%\meme_cutscene_skip\` at launch. And
UPX is always the last step: the next `cargo build` overwrites the exe uncompressed.

Tests (a fresh debug build is required: the launcher embeds the DLL via
`include_bytes!`, and without it `cargo test` cannot be built):

```sh
cargo build                       # the debug build creates the DLL payload
cargo test
cargo test --lib                  # state-machine logic only, no DLL build
```

## CI and releases

On a `v*` tag push, `.github/workflows/build.yml` builds the Windows artifact
(`build.ps1` → UPX → `out/meme_cutscene_skip.zip`), attaches it to a GitHub Release
and, when configured, uploads it to Yandex Object Storage.

For the S3 upload the repository must define (Settings → Secrets and
variables → Actions):

- **Variable** `S3_BUCKET` — bucket name; while it is unset, the S3 step is skipped;
- **Variable** `LOCKBOX_SECRET_ID` — id of the Yandex Lockbox secret holding the S3 keys;
- **Secret** `YC_SA_JSON_CREDENTIALS` — service account JSON key.

## Running

```sh
cargo run                 # find/start the game and inject
cargo run -- --follow     # same, plus print the mod log to the console
cargo run -- --kill-first --follow   # clean game start (for a fresh DLL)
```

Flags: `--exe <path>`, `--kill-first`, `--no-launch`, `--follow`,
`--timeout <sec>`. If the game is already running, the launcher injects into it.

**Live development loop.** `LoadLibraryW` does not call `DllMain` a second time, so
a freshly built DLL is not picked up by an already-loaded process — restart the game
for new code (`--kill-first`). We do not `FreeLibrary`/unload: hot DLL reloading
crashed the game in drmod-rs.

## Dependencies

- `windows` — Win32 API (memory, threads, ToolHelp, windows).
- MinHook — vendored in `vendor/minhook` (BSD-2-Clause, `LICENSE.txt`) to keep the
  repository self-contained.

## License

MIT — see [LICENSE](LICENSE).

The vendored MinHook under `vendor/minhook` is licensed separately under
BSD-2-Clause — see `vendor/minhook/LICENSE.txt`.

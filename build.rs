//! Builds the vendored MinHook (x86). Paths are relative to the crate: the
//! sources live in `vendor/minhook`, which keeps the crate self-contained (see
//! README).

use std::path::Path;

fn main() {
    let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("vendor/minhook/src");

    cc::Build::new()
        .file(src.join("buffer.c"))
        .file(src.join("hook.c"))
        .file(src.join("trampoline.c"))
        .file(src.join("hde/hde32.c"))
        .compile("libminhook.a");

    println!("cargo:rerun-if-changed=vendor/minhook/src");
}

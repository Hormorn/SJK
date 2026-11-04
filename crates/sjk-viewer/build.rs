//! Build settings of the client: the build's version, commit and commit time
//! (`scripts/build_version.rs`, shared with the dedicated server), and on
//! Windows the main thread's stack and the program's icon and version strings.
//!
//! The main thread builds the first world (the menu's map) and runs every frame;
//! its frames are large (`window_event` alone about 3 MB). Windows reserves 1 MiB
//! for it and macOS 8 MiB, which the world build alone came within 2 MiB of
//! (03/11/2026: a profile with JoF content overflowed 8 MiB at the menu's map), so
//! Windows and macOS binaries reserve 64 MiB, as the world-install worker does
//! (`session_transition::WORLD_INSTALL_STACK_BYTES`). Reserved pages that are never
//! touched cost nothing. Linux sets the same allowance at startup (within the
//! inherited hard limit) in `platform/main_stack.rs`. `sjk.exe` carries SJK's icon
//! (`assets/branding/sjk.ico`, which Explorer and shortcuts show) and names
//! itself "SJK" in its version strings, which Task Manager shows. Other
//! platforms have no executable icons.
//!
//! On Windows `sjk.exe` also exports NVIDIA's and AMD's hints to run on the fast
//! graphics card of a laptop that has two (`src/gpu_hints.c`, compiled here: the
//! client's Rust forbids the unmangled statics they need). `/INCLUDE` keeps the
//! otherwise unreferenced object in the link.

#[path = "../../scripts/build_version.rs"]
mod build_version;

mod stack_size;
use stack_size::MAIN_STACK_BYTES;
/// SJK's icon set, relative to this crate.
const ICON: &str = "../../assets/branding/sjk.ico";
/// The fast-card hints' source, relative to this crate, and the symbols it exports.
const GPU_HINTS: &str = "src/gpu_hints.c";
const GPU_HINT_SYMBOLS: [&str; 2] = [
    "NvOptimusEnablement",
    "AmdPowerXpressRequestHighPerformance",
];

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=stack_size.rs");
    println!("cargo:rerun-if-changed={ICON}");
    println!("cargo:rerun-if-changed=../../scripts/build_version.rs");
    println!("cargo:rerun-if-changed={GPU_HINTS}");
    let version = build_version::emit();
    let target_os = std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    if target_os == "macos" {
        println!("cargo:rustc-link-arg-bins=-Wl,-stack_size,{MAIN_STACK_BYTES:#x}");
    }
    if target_os != "windows" {
        return;
    }
    let target_env = std::env::var("CARGO_CFG_TARGET_ENV").unwrap_or_default();
    match target_env.as_str() {
        "msvc" => println!("cargo:rustc-link-arg-bins=/STACK:{MAIN_STACK_BYTES}"),
        "gnu" => println!("cargo:rustc-link-arg-bins=-Wl,--stack,{MAIN_STACK_BYTES}"),
        _ => {}
    }
    cc::Build::new().file(GPU_HINTS).compile("sjk_gpu_hints");
    for symbol in GPU_HINT_SYMBOLS {
        match target_env.as_str() {
            "msvc" => println!("cargo:rustc-link-arg-bins=/INCLUDE:{symbol}"),
            "gnu" => println!("cargo:rustc-link-arg-bins=-Wl,--undefined={symbol}"),
            _ => {}
        }
    }
    let mut resource = winresource::WindowsResource::new();
    resource
        .set_icon(ICON)
        .set("ProductName", "SJK")
        .set("FileDescription", "SJK")
        .set("ProductVersion", &version)
        .set("OriginalFilename", "sjk.exe");
    // A missing resource compiler costs the icon, not the build.
    if let Err(error) = resource.compile() {
        println!("cargo:warning=sjk is built without its icon: {error}");
    }
}

// Default allocator: mimalloc. `--features rusty-alloc` swaps it for
// rusty_alloc (pure-Rust mimalloc v2.4.5 remake, #5872); the default build
// is unchanged.
#[cfg(not(feature = "rusty-alloc"))]
#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

#[cfg(feature = "rusty-alloc")]
#[global_allocator]
static GLOBAL: rusty_alloc_api::RustyAlloc = rusty_alloc_api::RustyAlloc;

fn main() -> std::process::ExitCode {
    codewhale_tui::run(std::env::args().collect())
}

fn main() {
    #[cfg(feature = "runtime")]
    {
        let rustc = std::env::var("RUSTC").unwrap_or_else(|_| "rustc".into());
        let version = std::process::Command::new(&rustc)
            .arg("--version")
            .output()
            .ok()
            .and_then(|o| String::from_utf8(o.stdout).ok());

        match version {
            Some(v) if v.contains("nightly") || v.contains("dev") => {}
            Some(v) => {
                println!(
                    "cargo:warning=The `runtime` feature requires a nightly Rust compiler. Found: {}",
                    v.trim()
                );
                println!("cargo:warning=Switch to nightly with: rustup default nightly");
            }
            None => {
                println!("cargo:warning=Could not detect Rust toolchain. The `runtime` feature may not compile on stable.");
            }
        }
    }
}

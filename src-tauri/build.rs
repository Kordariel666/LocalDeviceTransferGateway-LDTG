fn main() {
    println!("cargo:rerun-if-env-changed=LDTG_TEST_RUNNER_MANIFEST");
    #[cfg(windows)]
    if std::env::var_os("LDTG_TEST_RUNNER_MANIFEST").is_some() {
        let manifest = std::path::PathBuf::from(
            std::env::var("CARGO_MANIFEST_DIR").expect("manifest directory"),
        )
        .join("windows")
        .join("test-runner.manifest");
        println!("cargo:rustc-link-arg=/MANIFEST:EMBED");
        println!("cargo:rustc-link-arg=/MANIFESTINPUT:{}", manifest.display());
        println!("cargo:rerun-if-changed={}", manifest.display());
    }
    tauri_build::build()
}

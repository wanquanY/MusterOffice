fn main() {
    // The binary is runnable only from its verified bundle: bin/worker + lib/.
    // No development machine path or system FFmpeg fallback is embedded.
    let os = std::env::var("CARGO_CFG_TARGET_OS").unwrap();
    match os.as_str() {
        "macos" => println!("cargo:rustc-link-arg=-Wl,-rpath,@executable_path/../lib"),
        "linux" => {
            println!("cargo:rustc-link-arg=-Wl,--disable-new-dtags");
            println!("cargo:rustc-link-arg=-Wl,-rpath,$ORIGIN/../lib");
        }
        _ => panic!("media worker target profile unavailable"),
    }
}

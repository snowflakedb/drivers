fn main() {
    // Add --hash-style=both for Linux targets for better compatibility.
    // This includes both GNU and SysV hash tables in the ELF binary,
    // ensuring the shared library works on both older and newer Linux systems.
    let target_os = std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();

    if target_os == "linux" {
        println!("cargo:rustc-link-arg=-Wl,--hash-style=both");
    }
}

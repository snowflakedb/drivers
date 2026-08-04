fn main() {
    // On macOS, Python extension modules leave Python symbols undefined at
    // link time - they are resolved at load time by the interpreter. Without
    // this flag the linker rejects the build with "symbol(s) not found".
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("macos") {
        println!("cargo:rustc-cdylib-link-arg=-undefined");
        println!("cargo:rustc-cdylib-link-arg=dynamic_lookup");
    }
}

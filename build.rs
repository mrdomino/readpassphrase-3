fn main() {
    println!("cargo:rerun-if-env-changed=CARGO_CFG_TARGET_OS");
    if std::env::var_os("CARGO_CFG_TARGET_OS").is_some_and(|os| os == "windows") {
        // Needs to be cfg directive since cc is an optional build dependency with this condition.
        #[cfg(target_os = "windows")]
        {
            cc::Build::new()
                .file("csrc/read-password-w32.c")
                .compile("read-password-w32");
            println!("cargo:rerun-if-changed=csrc/read-password-w32.c");
        }
    }
}

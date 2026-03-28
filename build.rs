use std::ffi::OsString;

fn main() {
    println!("cargo:rustc-check-cfg=cfg(raw_ffi)");

    if env_var_os("CARGO_FEATURE_EXTERNAL").is_some() {
        println!("cargo:rustc-cfg=raw_ffi");
        return;
    }

    let target_os = env_var_os("CARGO_CFG_TARGET_OS").unwrap_or_default();
    if target_os == "windows" {
        let _witness = env_var_os("CARGO_FEATURE_WINDOWS_VENDORED");
    }
    // Needs to be a cfg directive since cc is an optional build dependency with these conditions.
    #[cfg(all(target_os = "windows", feature = "windows-vendored"))]
    {
        cc::Build::new()
            .file("csrc/read-password-w32.c")
            .compile("read-password-w32");
        println!("cargo:rerun-if-changed=csrc/read-password-w32.c");
        println!("cargo:rustc-cfg=raw_ffi");
        return;
    }

    if env_var_os("CARGO_FEATURE_LIBBSD").is_some() {
        return;
    }

    if target_os == "macos"
        || target_os == "freebsd"
        || target_os == "netbsd"
        || target_os == "openbsd"
        || target_os == "dragonflybsd"
    {
        // *BSD ships with readpassphrase.
        println!("cargo:rustc-cfg=raw_ffi");
        return;
    }

    eprintln!("No readpassphrase implementation found.");
    std::process::exit(1);
}

fn env_var_os(key: &str) -> Option<OsString> {
    println!("cargo:rerun-if-env-changed={key}");
    std::env::var_os(key)
}

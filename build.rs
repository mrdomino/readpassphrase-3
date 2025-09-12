fn main() {
    // Check for a readpassphrase implementation in the following places in decreasing order of
    // preference:
    // 1. macOS libc.
    // 2. The libbsd static library.
    // 3. The Windows vendored source code on Windows.
    // 4. The non-Windows vendored source code from the dependent crate.
    //
    // If the implementation comes from the dependent crate, then we also need to set a cfg
    // directive to tell the library to use it.
    println!("cargo:rustc-check-cfg=cfg(use_tcm)");
    let mut found_readpassphrase = false;

    println!("cargo:rerun-if-env-changed=CARGO_CFG_TARGET_OS");
    let target_os = std::env::var_os("CARGO_CFG_TARGET_OS").unwrap_or_default();
    if target_os == "macos" {
        // macOS ships readpassphrase in its libc.
        found_readpassphrase = true;
    }

    println!("cargo:rerun-if-env-changed=CARGO_FEATURE_LIBBSD_STATIC");
    if !found_readpassphrase && std::env::var_os("CARGO_FEATURE_LIBBSD_STATIC").is_some() {
        // Rerun if any environment variable affecting pkg-config changes
        println!("cargo:rerun-if-env-changed=PKG_CONFIG_PATH");
        println!("cargo:rerun-if-env-changed=PKG_CONFIG_DEBUG_SPEW");
        println!("cargo:rerun-if-env-changed=PKG_CONFIG_TOP_BUILD_DIR");
        println!("cargo:rerun-if-env-changed=PKG_CONFIG_DISABLE_UNINSTALLED");
        println!("cargo:rerun-if-env-changed=PKG_CONFIG_ALLOW_SYSTEM_CFLAGS");
        println!("cargo:rerun-if-env-changed=PKG_CONFIG_ALLOW_SYSTEM_LIBS");
        println!("cargo:rerun-if-env-changed=PKG_CONFIG_SYSROOT_DIR");
        println!("cargo:rerun-if-env-changed=PKG_CONFIG_LIBDIR");

        match std::process::Command::new("pkg-config")
            .args(["--atleast-version", "0.9", "--exists", "--static", "libbsd"])
            .status()
        {
            Ok(status) if status.success() => {
                println!("cargo:rustc-link-lib=static:-bundle=bsd");
                found_readpassphrase = true;
            }
            Ok(_) => eprintln!("Warning: libbsd not found or version too old"),
            Err(e) => eprintln!("Warning: pkg-config failed: {e}"),
        }
    }

    println!("cargo:rerun-if-env-changed=CARGO_FEATURE_WINDOWS_VENDORED");
    // Needs to be a cfg directive since cc is an optional build dependency with these conditions.
    #[cfg(all(target_os = "windows", feature = "windows-vendored"))]
    {
        if !found_readpassphrase {
            cc::Build::new()
                .file("csrc/read-password-w32.c")
                .compile("read-password-w32");
            println!("cargo:rerun-if-changed=csrc/read-password-w32.c");
            found_readpassphrase = true;
        }
    }

    println!("cargo:rerun-if-env-changed=CARGO_FEATURE_LINUX_VENDORED");
    if !found_readpassphrase
        && target_os != "windows"
        && std::env::var_os("CARGO_FEATURE_LINUX_VENDORED").is_some()
    {
        found_readpassphrase = true;
        // Fetch readpassphrase out of the dependent crate.
        println!("cargo:rustc-cfg=use_tcm");
    }

    println!("cargo:rerun-if-env-changed=CARGO_FEATURE_EXTERNAL");
    if !found_readpassphrase && std::env::var_os("CARGO_FEATURE_EXTERNAL").is_none() {
        eprintln!("No readpassphrase implementation found.");
        std::process::exit(1);
    }
}

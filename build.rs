use std::ffi::OsString;

fn main() {
    println!("cargo:rustc-check-cfg=cfg(use_libbsd)");
    println!("cargo:rustc-check-cfg=cfg(use_tcm)");
    println!("cargo:rustc-check-cfg=cfg(use_external)");

    if env_var_os("CARGO_FEATURE_EXTERNAL").is_some() {
        println!("cargo:rustc-cfg=use_external");
        return;
    }

    let target_os = env_var_os("CARGO_CFG_TARGET_OS").unwrap_or_default();

    if target_os == "windows" {
        if env_var_os("CARGO_FEATURE_WINDOWS_VENDORED").is_some() {
            // Needs to be a cfg directive since cc is an optional build dependency with these conditions.
            #[cfg(all(target_os = "windows", feature = "windows-vendored"))]
            {
                cc::Build::new()
                    .file("csrc/read-password-w32.c")
                    .compile("read-password-w32");
            }
            println!("cargo:rerun-if-changed=csrc/read-password-w32.c");
            return;
        }
        panic!("unsupported platform/feature combo - try external or windows-vendored");
    }

    if env_var_os("CARGO_FEATURE_LIBBSD").is_some() {
        println!("cargo:rustc-cfg=use_libbsd");
        return;
    }

    if target_os == "linux" {
        if env_var_os("CARGO_FEATURE_LINUX_VENDORED").is_some() {
            println!("cargo:rustc-cfg=use_tcm");
            return;
        }
        panic!("unsupported platform/feature combo - try external or libbsd or linux-vendored");
    }

    if target_os == "macos"
        || target_os == "freebsd"
        || target_os == "netbsd"
        || target_os == "openbsd"
        || target_os == "dragonflybsd"
    {
        println!("cargo:rustc-cfg=use_external");
        return;
    }

    panic!("unsupported platform");
}

fn env_var_os(key: &str) -> Option<OsString> {
    println!("cargo:rerun-if-env-changed={key}");
    std::env::var_os(key)
}

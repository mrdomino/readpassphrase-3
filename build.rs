use std::ffi::OsString;

fn main() {
    println!("cargo:rustc-check-cfg=cfg(use_tcm)");
    println!("cargo:rustc-check-cfg=cfg(use_libbsd)");

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
        if use_external() {
            return;
        }
        panic!("unsupported platform/feature combo - try external or windows-vendored");
    }

    if target_os == "linux" {
        if use_libbsd() {
            return;
        }
        if use_linux_vendored() {
            return;
        }
        if use_external() {
            return;
        }
        panic!("unsupported platform/feature combo - try external or libbsd or linux-vendored");
    }

    if use_libbsd() {
        return;
    }

    if target_os == "macos"
        || target_os == "dragonflybsd"
        || target_os == "freebsd"
        || target_os == "netbsd"
        || target_os == "openbsd"
        || use_external()
    {
        return;
    }

    panic!("unsupported platform/feature combo - try external or libbsd");
}

fn env_var_os(key: &str) -> Option<OsString> {
    println!("cargo:rerun-if-env-changed={key}");
    std::env::var_os(key)
}

fn use_external() -> bool {
    env_var_os("CARGO_FEATURE_EXTERNAL").is_some()
}

fn use_libbsd() -> bool {
    if env_var_os("CARGO_FEATURE_LIBBSD").is_some() {
        println!("cargo:rustc-cfg=use_libbsd");
        return true;
    }
    false
}

fn use_linux_vendored() -> bool {
    if env_var_os("CARGO_FEATURE_LINUX_VENDORED").is_some() {
        println!("cargo:rustc-cfg=use_tcm");
        return true;
    }
    false
}

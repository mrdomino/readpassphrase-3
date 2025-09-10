// Copyright 2025 Steven Dee.
//
// This project is made available under a BSD-compatible license. See the
// LICENSE file in the project root for details.
//
// The readpassphrase source and header are copyright 2000-2002, 2007, 2010
// Todd C. Miller.

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    #[cfg(feature = "libbsd-static")]
    {
        // Rerun if any environment variable affecting pkg-config changes
        println!("cargo:rerun-if-env-changed=PKG_CONFIG_PATH");
        println!("cargo:rerun-if-env-changed=PKG_CONFIG_DEBUG_SPEW");
        println!("cargo:rerun-if-env-changed=PKG_CONFIG_TOP_BUILD_DIR");
        println!("cargo:rerun-if-env-changed=PKG_CONFIG_DISABLE_UNINSTALLED");
        println!("cargo:rerun-if-env-changed=PKG_CONFIG_ALLOW_SYSTEM_CFLAGS");
        println!("cargo:rerun-if-env-changed=PKG_CONFIG_ALLOW_SYSTEM_LIBS");
        println!("cargo:rerun-if-env-changed=PKG_CONFIG_SYSROOT_DIR");
        println!("cargo:rerun-if-env-changed=PKG_CONFIG_LIBDIR");

        if std::process::Command::new("pkg-config")
            .args(["--atleast-version", "0.9", "--exists", "libbsd"])
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
        {
            println!("cargo:rustc-link-lib=static:-bundle=bsd");
            return;
        }
    }
    #[cfg(all(target_os = "windows", feature = "vendored"))]
    {
        cc::Build::new()
            .file("csrc/read-password-w32.c")
            .compile("read-password-w32");
        println!("cargo:rerun-if-changed=csrc/read-password-w32.c");
    }
}

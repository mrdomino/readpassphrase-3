// Copyright 2025 Steven Dee.
//
// This project is made available under a BSD-compatible license. See the
// LICENSE file in the project root for details.
//
// The readpassphrase source and header are copyright 2000-2002, 2007, 2010
// Todd C. Miller.
use std::env;

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    #[cfg(not(feature = "cc"))]
    if env::var_os("CARGO_CFG_TARGET_OS").unwrap_or_default() == "linux" {
        println!("cargo:rustc-link-lib=static:-bundle=bsd");
    }
    #[cfg(feature = "cc")]
    {
        if env::var_os("CARGO_CFG_WINDOWS").is_some() {
            cc::Build::new()
                .file("csrc/read-password-w32.c")
                .compile("read-password-w32");
            println!("cargo:rerun-if-changed=csrc/read-password-w32.c");
        } else {
            cc::Build::new()
                .file("csrc/readpassphrase.c")
                .include("csrc")
                .compile("readpassphrase");
            println!("cargo:rerun-if-changed=csrc/readpassphrase.c");
            println!("cargo:rerun-if-changed=csrc/readpassphrase.h");
        }
    }
}

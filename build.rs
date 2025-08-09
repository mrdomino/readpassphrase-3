// Copyright 2025 Steven Dee.
//
// This project is made available under a BSD-compatible license. See the
// LICENSE file in the project root for details.
//
// The readpassphrase source and header are copyright 2000-2002, 2007, 2010
// Todd C. Miller.

use std::env;

fn main() {
    if env::var_os("CARGO_CFG_WINDOWS").is_some() {
        cc::Build::new()
            .file("csrc/read-password-w32.c")
            .compile("readpassphrase");
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

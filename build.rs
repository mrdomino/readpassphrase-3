// Copyright 2025 Steven Dee.
//
// This project is made available under a BSD-compatible license. See the
// LICENSE file in the project root for details.
//
// The readpassphrase source and header are copyright 2000-2002, 2007, 2010
// Todd C. Miller.

fn main() {
    #[cfg(feature = "cc")]
    {
        use std::env;

        if env::var_os("CARGO_CFG_WINDOWS").is_some() {
            cc::Build::new()
                .file("csrc/read-password-w32.c")
                .compile("read-password-w32");
        } else {
            cc::Build::new()
                .file("csrc/readpassphrase.c")
                .include("csrc")
                .compile("readpassphrase");
        }
    }
}

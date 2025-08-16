// Copyright 2025 Steven Dee.
//
// This project is made available under a BSD-compatible license. See the
// LICENSE file in the project root for details.
//
// The readpassphrase source and header are copyright 2000-2002, 2007, 2010
// Todd C. Miller.

use std::process::exit;

use readpassphrase_3::{Flags as RpFlags, PASSWORD_LEN, readpassphrase};
use zeroize::Zeroizing;

fn main() {
    let mut buf = Zeroizing::new(vec![0u8; PASSWORD_LEN]);
    let password = Zeroizing::new(
        readpassphrase(c"Password: ", &mut buf, RpFlags::empty())
            .expect("failed reading passphrase")
            .to_string(),
    );
    for _ in 0..5 {
        let confirm = readpassphrase(c"Confirmation: ", &mut buf, RpFlags::REQUIRE_TTY)
            .expect("failed reading confirmation");
        if *password == confirm {
            eprintln!("Passwords match.");
            return;
        }
        eprintln!("Passwords don’t match.");
    }
    eprintln!("Too many attempts.");
    exit(1);
}

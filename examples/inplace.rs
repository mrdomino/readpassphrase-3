// Copyright 2025 Steven Dee.
//
// This project is made available under a BSD-compatible license. See the
// LICENSE file in the project root for details.
//
// The readpassphrase source and header are copyright 2000-2002, 2007, 2010
// Todd C. Miller.

use readpassphrase_3::{RppFlags, readpassphrase};
use zeroize::Zeroizing;

fn main() {
    let mut buf = Zeroizing::new(vec![0u8; 256]);
    let password = readpassphrase(
        c"Password: ",
        &mut buf,
        RppFlags::FORCEUPPER | RppFlags::ECHO_ON,
    )
    .expect("failed reading passphrase");
    println!("{password}");
}

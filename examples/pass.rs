// Copyright 2025 Steven Dee.
//
// This project is made available under a BSD-compatible license. See the
// LICENSE file in the project root for details.
//
// The readpassphrase source and header are copyright 2000-2002, 2007, 2010
// Todd C. Miller.

use readpassphrase_3::{getpass, zeroize::Zeroize};

fn main() {
    let mut password = getpass(c"Password: ").expect("failed reading password");
    println!("{password}");
    password.zeroize();
}

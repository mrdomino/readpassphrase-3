// Copyright 2025 Steven Dee.
//
// This project is dual licensed under the MIT and Apache 2.0 licenses. See
// the LICENSE file in the project root for details.
//
// The readpassphrase source and header are copyright 2000-2002, 2007, 2010
// Todd C. Miller.

use readpassphrase_3::{RppFlags, readpassphrase};

fn main() {
    let password =
        readpassphrase(c"Password: ", RppFlags::default()).expect("failed reading password");
    println!("{password}");
}

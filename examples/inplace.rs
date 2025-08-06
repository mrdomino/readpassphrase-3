// Copyright 2025 Steven Dee.
//
// This project is dual licensed under the MIT and Apache 2.0 licenses. See
// the LICENSE file in the project root for details.
//
// The readpassphrase source and header are copyright 2000-2002, 2007, 2010
// Todd C. Miller.

use readpassphrase_3::{RppFlags, readpassphrase_inplace};
#[cfg(feature = "zeroize")]
use zeroize::Zeroizing;

fn main() {
    #[allow(unused_mut)]
    let mut buf = vec![0u8; 256];
    #[cfg(feature = "zeroize")]
    let mut buf = Zeroizing::new(buf);
    let password = readpassphrase_inplace(
        c"Password: ",
        &mut buf,
        RppFlags::FORCEUPPER | RppFlags::ECHO_ON,
    )
    .expect("failed reading passphrase");
    println!("{password}");
}

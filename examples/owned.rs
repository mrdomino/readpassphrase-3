// Copyright 2025 Steven Dee.
//
// This project is made available under a BSD-compatible license. See the
// LICENSE file in the project root for details.
//
// The readpassphrase source and header are copyright 2000-2002, 2007, 2010
// Todd C. Miller.

use readpassphrase_3::{readpassphrase, readpassphrase_owned, Error, Flags, PASSWORD_LEN};
use zeroize::{Zeroize, Zeroizing};

fn main() -> Result<(), Error> {
    let mut buf = Zeroizing::new(Some(vec![0u8; PASSWORD_LEN]));
    let pass = Zeroizing::new(
        readpassphrase(c"Password: ", buf.as_deref_mut().unwrap(), Flags::ECHO_ON)?.to_string(),
    );
    let mut buf = buf.take();
    loop {
        let mut res =
            readpassphrase_owned(c"Confirmation: ", buf.take().unwrap(), Flags::REQUIRE_TTY)?;
        if *pass == res {
            res.zeroize();
            break;
        }
        buf = Some(res.into_bytes());
    }
    Ok(())
}

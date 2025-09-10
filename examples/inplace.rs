use std::process::exit;

use readpassphrase_3::{readpassphrase, Flags as RpFlags, PASSWORD_LEN};
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

use readpassphrase_3::{readpassphrase, readpassphrase_into, Error, Flags, PASSWORD_LEN};
use zeroize::{Zeroize, Zeroizing};

fn main() -> Result<(), Error> {
    let mut buf = Zeroizing::new(Some(vec![0u8; PASSWORD_LEN]));
    let pass = Zeroizing::new(
        readpassphrase(c"Password: ", buf.as_deref_mut().unwrap(), Flags::ECHO_ON)?.to_string(),
    );
    let mut buf = buf.take();
    loop {
        buf = Some(
            match readpassphrase_into(c"Confirmation: ", buf.take().unwrap(), Flags::REQUIRE_TTY) {
                Ok(mut s) if *pass == s => {
                    s.zeroize();
                    break;
                }
                Ok(s) => s.into_bytes(),
                Err(e) => match e.error() {
                    Error::Io(_) => return Err(e.into()),
                    Error::Utf8(_) => {
                        eprintln!("decode error: {e}");
                        e.into_bytes()
                    }
                },
            },
        );
    }
    Ok(())
}

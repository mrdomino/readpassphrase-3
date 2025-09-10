use readpassphrase_3::{readpassphrase, readpassphrase_into, Error, Flags, PASSWORD_LEN};
use zeroize::{Zeroize, Zeroizing};

fn main() -> Result<(), Error> {
    let mut buf = Zeroizing::new(Some(vec![0u8; PASSWORD_LEN]));
    let pass = Zeroizing::new(
        readpassphrase(c"Password: ", buf.as_deref_mut().unwrap(), Flags::ECHO_ON)?.to_string(),
    );
    let mut buf = buf.take();
    loop {
        let mut res =
            readpassphrase_into(c"Confirmation: ", buf.take().unwrap(), Flags::REQUIRE_TTY)?;
        if *pass == res {
            res.zeroize();
            break;
        }
        buf = Some(res.into_bytes());
    }
    Ok(())
}

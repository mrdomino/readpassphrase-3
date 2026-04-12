use readpassphrase_3::{Zeroize, getpass};

fn main() {
    let mut password = getpass(c"Password: ").expect("failed reading password");
    println!("{password:?}");
    password.zeroize();
}

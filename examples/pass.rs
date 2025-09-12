use readpassphrase_3::{getpass, Zeroize};

fn main() {
    let mut password = getpass(c"Password: ").expect("failed reading password");
    println!("{password:?}");
    password.zeroize();
}

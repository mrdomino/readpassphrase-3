// Copyright 2025 Steven Dee
//
// Licensed under the [Apache License, Version 2.0][0] or the [MIT license][1],
// at your option.
//
// THE SOFTWARE IS PROVIDED “AS IS”, WITHOUT WARRANTY OF ANY KIND, EXPRESS
// OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL
// THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
// SOFTWARE.
//
// [0]: https://www.apache.org/licenses/LICENSE-2.0
// [1]: https://opensource.org/licenses/MIT

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

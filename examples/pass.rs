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

use readpassphrase_3::{getpass, Zeroize};

fn main() {
    let mut password = getpass(c"Password: ").expect("failed reading password");
    println!("{password}");
    password.zeroize();
}

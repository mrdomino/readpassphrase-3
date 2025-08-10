# readpassphrase-3
This crate endeavors to expose a thin Rust wrapper around the C [`readpassphrase(3)`][0] function for reading passphrases on the console in CLI programs.

This library uses a few third-party dependencies: flags to `readpassphrase` are implemented via the [bitflags][1] library, native builds are done via [cc][2], and memory zeroing can optionally be done by [zeroize][3]. To try to reduce churn in this library itself, we do not lock the versions of these dependencies; it is recommended that you vet their current versions yourself for compromises or software supply chain attacks. If you would rather not do that (or if you need support for wasm), consider instead using the excellent [rpassword][4] crate, which ships without external dependencies.

# Usage
Add this crate to your project with `cargo add readpassphrase-3`. If you would like memory zeroing to be done by [zeroize][3], then you can instead say: `cargo add readpassphrase-3 -F zeroize`.

See <https://docs.rs/readpassphrase-3> for documentation and examples.

# Features
- `cc`, enabled by default, compiles and uses a vendored version of the `readpassphrase` C function. (You may wish to turn this off to use the version from your system’s libc, or to compile and link the C function through other means.)
- `zeroize` uses [zeroize][3] to zero memory internally (otherwise a minimal in-crate version is used.)

# NFAQ

## I’m getting a “mismatched types” error!
That’s not a question, but it’s okay. You are probably passing a Rust `&str` as the prompt argument. To avoid needing to take a dynamically allocated string or make a copy of the prompt on every call, this library takes a [`&CStr`][5] (i.e. a null-terminated span of characters) as its prompt argument.

If you’re passing a literal string, you can just prepend `c` to your string:
```rust
let _ = getpass(c"Prompt: ")?;
//              ^
//              |
//              like this
```

## Why is this named `readpassphrase-3`?
There is already an unmaintained [readpassphrase][6] crate that was not to my liking. Rather than try to invent a new name for this standard C function, I decided to pick a number. The number I picked, 3, corresponds to the [“library calls” man section][7], in which readpassphrase’s man page is located.

[0]: https://man.openbsd.org/readpassphrase
[1]: https://crates.io/crates/bitflags
[2]: https://crates.io/crates/cc
[3]: https://crates.io/crates/zeroize
[4]: https://crates.io/crates/rpassword
[5]: https://doc.rust-lang.org/std/ffi/struct.CStr.html
[6]: https://crates.io/crates/readpassphrase
[7]: https://man7.org/linux/man-pages/man7/man-pages.7.html

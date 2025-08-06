# readpassphrase-3

This crate endeavors to expose a thin Rust wrapper around the OpenBSD [`readpassphrase(3)`][0] function. Three interfaces are exposed:
1. `readpassphrase`, which allocates and returns its own fixed-size buffer for the passphrase;
2. `readpassphrase_buf`, which takes a preallocated buffer that it consumes and returns as the output `String`; and
3. `readpassphrase_inplace`, which takes a buffer as a byte slice and returns a `&str` in that buffer.

These may be customized using `RppFlags`, which expose the original API’s flags.

This library uses a couple of third-party dependencies: `RppFlags` is implemented via the [bitflags][1] library, and memory zeroing is by default done via [zeroize][3]. To try to reduce churn in this library itself, and dependencies on multiple versions of libraries in dependent packages, we do not lock the versions of these dependencies; it is recommended that you vet their current versions yourself to guard against software supply chain attacks. If you would rather not do that, consider instead using the excellent [rpassword][4] crate, which vendors its own dependencies (and supports Windows!)

# NFAQ

## I’m getting a “mismatched types” error!

That’s not a question, but it’s okay. You are probably passing a Rust `&str` as the prompt argument. To avoid needing to take a dynamically allocated string or make a copy of the prompt on every call, this library takes a [`&CStr`][5] (i.e. a null-terminated span of characters) as its prompt argument.

If you’re passing a literal string, you can just prepend `c` to your string:

```rust
let _ = readpassphrase(c"Prompt: ", RppFlags::default())?;
//                     ^
//                     |
//                     like this
```

## Why is this named `readpassphrase-3`?

There is already an unmaintained [readpassphrase][6] crate that was not to my liking. Rather than try to invent a new name for this standard C function, I decided to pick a number. The number I picked, 3, corresponds to the [“library calls” man section][7], in which readpassphrase’s man page is located.

## Will this ever support Windows?

[Probably not][8].

[0]: https://man.openbsd.org/readpassphrase
[1]: https://crates.io/crates/bitflags
[2]: https://docs.rs/thiserror/latest/thiserror/
[3]: https://crates.io/crates/zeroize
[4]: https://crates.io/crates/rpassword
[5]: https://doc.rust-lang.org/std/ffi/struct.CStr.html
[6]: https://crates.io/crates/readpassphrase
[7]: https://man7.org/linux/man-pages/man7/man-pages.7.html
[8]: https://github.com/mrdomino/readpassphrase-3/pull/1

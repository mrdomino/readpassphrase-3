# readpassphrase-3
This crate endeavors to expose a thin Rust wrapper around the C [`readpassphrase(3)`][0] function for reading passphrases on the console in CLI programs.

It uses a few third-party dependencies: flags to `readpassphrase` are implemented via the [`bitflags`][1] library, native builds are done via [`cc`][2], and memory zeroing can optionally be done by [`zeroize`][3]. To try to reduce churn in this library itself, we do not lock the versions of these dependencies; it is recommended that you vet their current versions yourself for compromises or software supply chain attacks. If you would rather not do that (or if you need support for wasm), consider instead using the excellent [`rpassword`][4] crate, which ships without external dependencies.

# Usage
Add this crate to your project:
```sh
cargo add readpassphrase-3
```
If your project is using [`zeroize`][3], you can instead do:
```sh
cargo add readpassphrase-3 -F zeroize
```

See <https://docs.rs/readpassphrase-3> for documentation and examples.

# Crate Features
- `vendored`, enabled by default, compiles and uses a vendored version of the `readpassphrase` C function. (You may wish to turn this off to use the version from your system’s libc, or to compile and link the C function through other means.)
- `libbsd-static` uses `readpassphrase` from the `libbsd` system library statically, i.e. without incurring a runtime dependency.
- `zeroize` uses [`zeroize`][3] to zero memory internally (otherwise a minimal in-crate version is used.)

# NFAQ

## Why use this?
[`readpassphrase(3)`][0] is a standard function that exists in many platforms’ libc implementations. It has had a lot of miles put on it; it is well-tested and works even under conditions like suspend/resume with `C-z` / `fg`, keeping echo off and so forth.

As well, `readpassphrase(3)` —and the interfaces this library exposes to it— does not allocate extra memory, making it relatively easy to be sure that you have zeroed all copies of your passwords after use. As long as you zero the memory you own, either the buffer you pass in to the non-owned `readpassphrase` or the `String` you receive from the owned `getpass`, you’re good.

## Why not use this?
This crate requires either a `readpassphrase(3)` in the libc on your target platform or a build-time dependency on a C compiler; if you do not wish to take that on, then you should look elsewhere.

## I’m getting a “mismatched types” error!
That’s not a question, but it’s okay. You are probably passing a Rust `&str` as the prompt argument. To avoid needing to take a dynamically allocated string or make a copy of the prompt on every call, this library takes a [`&CStr`][5] (i.e. a null-terminated span of characters) as its prompt argument.

If you’re passing a literal string, you can just prepend `c` to your string:
```rust
let _ = getpass(c"Prompt: ")?;
//              ^
//              |
//              like this
```

If you need a dynamic prompt, look at [`CString`][6].

## Why is this named `readpassphrase-3`?
There is already an unmaintained [`readpassphrase`][7] crate that was not to my liking. Rather than try to invent a new name for this standard C function, I decided to pick a number. The number I picked, 3, corresponds to the [“library calls” man section][8], in which readpassphrase’s man page is located.

[0]: https://man.openbsd.org/readpassphrase
[1]: https://crates.io/crates/bitflags
[2]: https://crates.io/crates/cc
[3]: https://crates.io/crates/zeroize
[4]: https://crates.io/crates/rpassword
[5]: https://doc.rust-lang.org/std/ffi/struct.CStr.html
[6]: https://doc.rust-lang.org/std/ffi/struct.CString.html
[7]: https://crates.io/crates/readpassphrase
[8]: https://man7.org/linux/man-pages/man7/man-pages.7.html

# readpassphrase-2

This crate endeavors to expose a thin wrapper around the OpenBSD [`readpassphrase(3)`][0] function. Two interfaces are exposed:
1. `readpassphrase`, which allocates and returns its own fixed-size buffer for the passphrase; and
2. `readpassphrase_buf`, which takes a preallocated buffer that it consumes and returns as the output `String`.

These may be customized using `RppFlags`, which expose the original API’s flags using the [bitflags][1] library.

Errors are exposed via [thiserror][2], and memory zeroing is done via [zeroize][3].
To try to reduce churn in this library itself and dependencies on multiple versions of libraries in dependent packages,
we do not lock the versions of these dependencies; it is recommended that you vet their current versions yourself
to guard against software supply chain attacks. If you would rather not do that, consider instead using the
excellent [rpassword][4] crate, which vendors its own dependencies.

[0]: https://man.openbsd.org/readpassphrase
[1]: https://crates.io/crates/bitflags
[2]: https://docs.rs/thiserror/latest/thiserror/
[3]: https://crates.io/crates/zeroize
[4]: https://crates.io/crates/rpassword

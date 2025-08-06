# readpassphrase-3

This crate endeavors to expose a thin wrapper around the OpenBSD [`readpassphrase(3)`][0] function. Two interfaces are exposed:
1. `readpassphrase`, which allocates and returns its own fixed-size buffer for the passphrase; and
2. `readpassphrase_buf`, which takes a preallocated buffer that it consumes and returns as the output `String`.

These may be customized using `RppFlags`, which expose the original API’s flags using the [bitflags][1] library.

Errors are exposed via [thiserror][2], and memory zeroing is done via [zeroize][3].
To try to reduce churn in this library itself and dependencies on multiple versions of libraries in dependent packages,
we do not lock the versions of these dependencies; it is recommended that you vet their current versions yourself
to guard against software supply chain attacks. If you would rather not do that, consider instead using the
excellent [rpassword][4] crate, which vendors its own dependencies.

# NFAQ

## Why is this named `readpassphrase-3`?

There is already an unmaintained [readpassphrase][5] crate that was not to my liking.
Rather than try to invent a new name for this standard C function, I decided to pick a number.
The number I picked, 3, corresponds to the [“library calls” man section][6], in which readpassphrase’s man page is located.

[0]: https://man.openbsd.org/readpassphrase
[1]: https://crates.io/crates/bitflags
[2]: https://docs.rs/thiserror/latest/thiserror/
[3]: https://crates.io/crates/zeroize
[4]: https://crates.io/crates/rpassword
[5]: https://crates.io/crates/readpassphrase
[6]: https://man7.org/linux/man-pages/man7/man-pages.7.html

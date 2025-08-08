// Copyright 2025 Steven Dee.
//
// This project is made available under a BSD-compatible license. See the
// LICENSE file in the project root for details.
//
// The readpassphrase source and header are copyright 2000-2002, 2007, 2010
// Todd C. Miller.

//! This library endeavors to expose a thin wrapper around OpenBSD’s [`readpassphrase(3)`][0]
//! function.
//!
//! Three different interfaces are exposed; for most purposes, you will want to use either
//! [`getpass`] (for simple password entry) or [`readpassphrase`] (when you need flags from
//! `readpassphrase(3)` or need more control over the memory.)
//!
//! The [`readpassphrase_owned`] function is a bit more niche; it may be used when you need a
//! [`String`] output but need to pass flags or control the buffer size (vs [`getpass`].)
//!
//! [0]: https://man.openbsd.org/readpassphrase

use std::{
    ffi::{CStr, FromBytesUntilNulError},
    fmt::Display,
    io,
    str::Utf8Error,
};

use bitflags::bitflags;
#[cfg(feature = "zeroize")]
use zeroize::Zeroize;

pub const PASSWORD_LEN: usize = 256;

bitflags! {
    /// Flags for controlling readpassphrase
    pub struct RppFlags: i32 {
        /// Turn off echo (default)
        const ECHO_OFF    = 0x00;
        /// Leave echo on
        const ECHO_ON     = 0x01;
        /// Fail if there is no tty
        const REQUIRE_TTY = 0x02;
        /// Force input to lower case
        const FORCELOWER  = 0x04;
        /// Force input to upper case
        const FORCEUPPER  = 0x08;
        /// Strip the high bit from input
        const SEVENBIT    = 0x10;
        /// Read from stdin, not /dev/tty
        const STDIN       = 0x20;
    }
}

impl Default for RppFlags {
    fn default() -> Self {
        Self::ECHO_OFF
    }
}

/// Errors that can occur in readpassphrase
#[derive(Debug)]
pub enum Error {
    /// `readpassphrase(3)` itself encountered an error
    Io(io::Error),
    /// The entered password did not parse as UTF-8
    Utf8(Utf8Error),
    /// The buffer did not contain a null terminator
    CStr(FromBytesUntilNulError),
}

/// Reads a passphrase using `readpassphrase(3)`.
///
/// This function reads a passphrase of up to `buf.len() - 1` bytes. If the entered passphrase is
/// longer, it will be truncated.
///
/// # Security
/// The passed buffer might contain sensitive data, even if this function returns an error (for
/// example, if the contents are not valid UTF-8.) It is often considered good practice to zero
/// this memory after you’re done with it, for example by using [`zeroize`]:
/// ```no_run
/// # use readpassphrase_3::{PASSWORD_LEN, Error, RppFlags, readpassphrase};
/// use zeroize::Zeroizing;
/// # fn main() -> Result<(), Error> {
/// let mut buf = Zeroizing::new(vec![0u8; PASSWORD_LEN]);
/// let pass = readpassphrase(c"Pass: ", &mut buf, RppFlags::default())?;
/// # Ok(())
/// # }
/// ```
pub fn readpassphrase<'a>(
    prompt: &CStr,
    buf: &'a mut [u8],
    flags: RppFlags,
) -> Result<&'a str, Error> {
    unsafe {
        let res = ffi::readpassphrase(
            prompt.as_ptr(),
            buf.as_mut_ptr().cast(),
            buf.len(),
            flags.bits(),
        );
        if res.is_null() {
            return Err(io::Error::last_os_error().into());
        }
    }
    Ok(CStr::from_bytes_until_nul(buf)?.to_str()?)
}

/// Reads a passphrase using `readpassphrase(3)`, returning it as a [`String`].
///
/// Internally, this function uses a buffer of [`PASSWORD_LEN`] bytes, allowing for passwords up to
/// `PASSWORD_LEN - 1` characters (accounting for the C null terminator.) If the entered passphrase
/// is longer, it will be truncated.
///
/// The passed flags are always the defaults, i.e., [`RppFlags::ECHO_OFF`].
///
/// # Security
/// If the [`zeroize`] feature of this crate is disabled, then this function can leak sensitive
/// data on failure, e.g. if the entered passphrase is not valid UTF-8. There is no way around this
/// (other than using the default `zeroize` feature), so if you must turn that feature off and are
/// concerned about this, then you should use the [`readpassphrase`] function instead.
///
/// The returned `String` is owned by the caller, and therefore it is the caller’s responsibility
/// to clear it when you are done with it, for example by using [`zeroize`]:
/// ```no_run
/// # use readpassphrase_3::{Error, getpass};
/// use zeroize::Zeroizing;
/// # fn main() -> Result<(), Error> {
/// let pass = Zeroizing::new(getpass(c"Pass: ")?);
/// # Ok(())
/// # }
/// ```
pub fn getpass(prompt: &CStr) -> Result<String, Error> {
    #[allow(unused_mut, unused_variables)]
    readpassphrase_owned(prompt, vec![0u8; PASSWORD_LEN], RppFlags::empty()).map_err(
        |(e, mut buf)| {
            #[cfg(feature = "zeroize")]
            buf.zeroize();
            e
        },
    )
}

/// Reads a passphrase using `readpassphrase(3)` using the passed buffer’s memory.
///
/// This function reads a passphrase of up to `buf.len() - 1` bytes. If the entered passphrase is
/// longer, it will be truncated.
///
/// The returned [`String`] uses `buf`’s memory; on failure, this memory is returned to the caller in
/// the second argument of the `Err` tuple.
///
/// # Security
/// The returned `String` is owned by the caller, and therefore it is the caller’s responsibility
/// to clear it when you are done with it. You may also wish to zero the returned buffer on error,
/// as it may still contain sensitive data, for example if the password was not valid UTF-8.
///
/// This can be done via [`zeroize`], e.g.:
/// ```no_run
/// # use readpassphrase_3::{PASSWORD_LEN, Error, RppFlags, readpassphrase_owned};
/// use zeroize::{Zeroizing, Zeroize};
/// # fn main() -> Result<(), Error> {
/// let buf = vec![0u8; PASSWORD_LEN];
/// let pass = Zeroizing::new(
///     readpassphrase_owned(c"Pass: ", buf, RppFlags::default())
///         .map_err(|(e, mut buf)| { buf.zeroize(); e })?
/// );
/// # Ok(())
/// # }
/// ```
pub fn readpassphrase_owned(
    prompt: &CStr,
    mut buf: Vec<u8>,
    flags: RppFlags,
) -> Result<String, (Error, Vec<u8>)> {
    match readpassphrase(prompt, &mut buf, flags) {
        Ok(res) => {
            let len = res.len();
            buf.truncate(len);
            Ok(unsafe { String::from_utf8_unchecked(buf) })
        }

        Err(e) => Err((e, buf)),
    }
}

impl From<io::Error> for Error {
    fn from(value: io::Error) -> Self {
        Error::Io(value)
    }
}

impl From<Utf8Error> for Error {
    fn from(value: Utf8Error) -> Self {
        Error::Utf8(value)
    }
}

impl From<FromBytesUntilNulError> for Error {
    fn from(value: FromBytesUntilNulError) -> Self {
        Error::CStr(value)
    }
}

impl core::error::Error for Error {
    fn source(&self) -> Option<&(dyn core::error::Error + 'static)> {
        match self {
            Error::Io(e) => Some(e),
            Error::Utf8(e) => Some(e),
            Error::CStr(e) => Some(e),
        }
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Io(e) => e.fmt(f),
            Error::Utf8(e) => e.fmt(f),
            Error::CStr(e) => e.fmt(f),
        }
    }
}

mod ffi {
    use std::ffi::{c_char, c_int};

    unsafe extern "C" {
        pub(crate) unsafe fn readpassphrase(
            prompt: *const c_char,
            buf: *mut c_char,
            bufsiz: usize,
            flags: c_int,
        ) -> *mut c_char;
    }
}

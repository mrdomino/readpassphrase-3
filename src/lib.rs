// Copyright 2025
//	Steven Dee
//
// Redistribution and use in source and binary forms, with or without
// modification, are permitted provided that the following conditions
// are met:
//
// Redistributions of source code must retain the above copyright
// notice, this list of conditions and the following disclaimer.
//
// THIS SOFTWARE IS PROVIDED BY STEVEN DEE “AS IS” AND ANY EXPRESS
// OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED TO, THE IMPLIED
// WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE
// ARE DISCLAIMED. IN NO EVENT SHALL STEVEN DEE BE LIABLE FOR ANY
// DIRECT, INDIRECT, INCIDENTAL, SPECIAL, EXEMPLARY, OR CONSEQUENTIAL
// DAMAGES (INCLUDING, BUT NOT LIMITED TO, PROCUREMENT OF SUBSTITUTE
// GOODS OR SERVICES; LOSS OF USE, DATA, OR PROFITS; OR BUSINESS
// INTERRUPTION) HOWEVER CAUSED AND ON ANY THEORY OF LIABILITY, WHETHER
// IN CONTRACT, STRICT LIABILITY, OR TORT (INCLUDING NEGLIGENCE OR
// OTHERWISE) ARISING IN ANY WAY OUT OF THE USE OF THIS SOFTWARE, EVEN
// IF ADVISED OF THE POSSIBILITY OF SUCH DAMAGE.

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
//! If the passphrases you read are sensitive data, then it is usually advised to zero their memory
//! afterwards. You can do this with a crate like [`zeroize`] or with the provided
//! [`explicit_bzero`] function in this crate.
//!
//! To read a passphrase from the console:
//! ```no_run
//! # use readpassphrase_3::{explicit_bzero, getpass};
//! let pass = getpass(c"password: ").unwrap();
//! // do_something_with(&pass);
//! explicit_bzero(pass);
//! ```
//!
//! To control the buffer size or (on non-Windows) flags:
//! ```no_run
//! # use readpassphrase_3::{RppFlags, readpassphrase};
//! # let mut buf = vec![0u8; 1];
//! let pass = readpassphrase(c"pass: ", &mut buf, RppFlags::ECHO_ON).unwrap();
//! ```
//!
//! To do so while transferring ownership:
//! ```no_run
//! # use readpassphrase_3::{Error, RppFlags, clear_b, readpassphrase_owned};
//! # fn main() -> Result<(), Error> {
//! # let buf = vec![0u8; 1];
//! let pass = readpassphrase_owned(c"pass: ", buf, RppFlags::empty()).map_err(clear_b)?;
//! # Ok(())
//! # }
//! ```
//!
//! # “Mismatched types” errors
//! The prompt strings in this API are references to [CStr], not [str]. This is because the
//! underlying C function assumes that the prompt is a null-terminated string; were we to take
//! `&str` instead of `&CStr`, we would need to make a copy of the prompt on every call.
//!
//! Most of the time, your prompts will be string literals; you can ask Rust to give you a `&CStr`
//! literal by simply prepending `c` to the string:
//! ```no_run
//! # use readpassphrase_3::{Error, getpass};
//! # fn main() -> Result<(), Error> {
//! let _ = getpass(c"pass: ")?;
//! //              ^
//! //              |
//! //              like this
//! # Ok(())
//! # }
//! ```
//!
//! # Windows Limitations
//! The Windows implementation of `readpassphrase(3)` that we are using does not yet support UTF-8
//! in prompts; they must be ASCII. It also does not yet support flags, and always behaves as
//! though called with [`RppFlags::empty()`].
//!
//! [0]: https://man.openbsd.org/readpassphrase

use std::{
    ffi::{CStr, FromBytesUntilNulError},
    fmt::Display,
    io, mem,
    str::Utf8Error,
};

use bitflags::bitflags;

/// Length of buffer used in [`getpass`].
///
/// Because [`ffi::readpassphrase`] null-terminates its string, the actual maximum password length
/// for [`getpass`] is 255.
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
/// The passed flags are always the defaults, i.e., [`RppFlags::default()`].
///
/// # Security
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
    readpassphrase_owned(prompt, vec![0u8; PASSWORD_LEN], RppFlags::empty()).map_err(clear_b)
}

/// Reads a passphrase using `readpassphrase(3)` by reusing the passed buffer’s memory.
///
/// This function reads a passphrase of up to `buf.capacity() - 1` bytes. If the entered passphrase
/// is longer, it will be truncated.
///
/// The returned [`String`] uses `buf`’s memory; on failure, this memory is returned to the caller
/// in the second argument of the `Err` tuple with its length set to 0.
///
/// # Security
/// The returned `String` is owned by the caller, and it is the caller’s responsibility to clear
/// it. It is also the caller’s responsibility to clear the buffer returned on error, as it may
/// still contain sensitive data, for example if the password was not valid UTF-8.
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
    readpassphrase_mut(prompt, &mut buf, flags).map_err(|e| {
        buf.clear();
        (e, buf)
    })
}

/// Reads a passphrase into `buf`’s maybe-uninitialized capacity and returns it as a `String`
/// reusing `buf`’s memory on success. This function serves to make it possible to write
/// `readpassphrase_owned` without either pre-initializing the buffer or invoking undefined
/// behavior by constructing a maybe-uninitialized slice.
fn readpassphrase_mut(prompt: &CStr, buf: &mut Vec<u8>, flags: RppFlags) -> Result<String, Error> {
    unsafe {
        let res = ffi::readpassphrase(
            prompt.as_ptr(),
            buf.as_mut_ptr().cast(),
            buf.capacity(),
            flags.bits(),
        );
        if res.is_null() {
            return Err(io::Error::last_os_error().into());
        }
        let res = CStr::from_ptr(res).to_str()?;
        buf.set_len(res.len());
        Ok(String::from_utf8_unchecked(mem::take(buf)))
    }
}

/// Convenience function to zero the memory from `readpassphrase_owned` on error.
///
/// Usage:
/// ```no_run
/// # use readpassphrase_3::{Error, PASSWORD_LEN, RppFlags, readpassphrase_owned, clear_b};
/// # fn main() -> Result<(), Error> {
/// let buf = vec![0u8; PASSWORD_LEN];
/// let pass = readpassphrase_owned(c"pass: ", buf, RppFlags::empty()).map_err(clear_b)?;
/// # Ok(())
/// # }
/// ```
pub fn clear_b<A>((a, mut b): (A, Vec<u8>)) -> A {
    explicit_bzero_vec(&mut b);
    a
}

/// Securely zero the memory in `s`.
///
/// This function clears the full capacity of `s` by writing zeroes to it, thereby erasing any
/// sensitive data in `s`. It should be called to clear any sensitive passphrases once they are no
/// longer in use.
///
/// If the `zeroize` feature is enabled, this internally uses [`zeroize::Zeroize`].
pub fn explicit_bzero(s: String) {
    let mut buf = Vec::from(s);
    explicit_bzero_vec(&mut buf);
}

/// Securely zero the memory in `buf`.
///
/// This function clears the full capacity of `buf` by writing zeroes to it, thereby erasing any
/// sensitive data in `buf`. It should be called to clear any sensitive passphrases once they are
/// no longer in use.
///
/// If the `zeroize` feature is enabled, this internally uses [`zeroize::Zeroize`].
pub fn explicit_bzero_vec(buf: &mut Vec<u8>) {
    #[cfg(feature = "zeroize")]
    {
        use zeroize::Zeroize;
        buf.zeroize();
    }
    #[cfg(not(feature = "zeroize"))]
    {
        buf.clear();
        buf.spare_capacity_mut()
            .fill(std::mem::MaybeUninit::zeroed());
        unsafe {
            core::arch::asm!(
                "/* {ptr} */",
                ptr = in(reg) buf.as_ptr(),
                options(nostack, readonly, preserves_flags),
            );
        }
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

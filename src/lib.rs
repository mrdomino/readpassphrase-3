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

//! Lightweight, easy-to-use wrapper around the C [`readpassphrase(3)`][0] function.
//!
//! # Usage
//! For the simplest of cases, where you would just like to read a password from the console into a
//! [`String`] to use elsewhere, you can use [`getpass`]:
//! ```no_run
//! use readpassphrase_3::getpass;
//! let _ = getpass(c"Enter your password: ").expect("failed reading password");
//! ```
//!
//! If you need to pass [`Flags`] or to control the buffer size, then you can use
//! [`readpassphrase`] or [`readpassphrase_owned`] depending on your ownership requirements:
//! ```no_run
//! use readpassphrase_3::{Flags, readpassphrase};
//! let mut buf = vec![0u8; 256];
//! let _ = readpassphrase(c"Password: ", &mut buf, Flags::default()).unwrap();
//!
//! use readpassphrase_3::readpassphrase_owned;
//! let _ = readpassphrase_owned(c"Pass: ", buf, Flags::FORCELOWER).unwrap();
//! ```
//!
//! # Security
//! Sensitive data should be zeroed as soon as possible to avoid leaving it visible in the
//! process’s address space. It is your job to ensure that this is done with the data you own, i.e.
//! any [`Vec`] passed to [`readpassphrase`] or any [`String`] received from [`getpass`] or
//! [`readpassphrase_owned`].
//!
//! This crate ships with a minimal [`Zeroize`] trait that may be used for this purpose:
//! ```no_run
//! # use readpassphrase_3::{Flags, getpass, readpassphrase, readpassphrase_owned};
//! use readpassphrase_3::Zeroize;
//! let mut pass = getpass(c"password: ").unwrap();
//! // do_something_with(&pass);
//! pass.zeroize();
//!
//! let mut buf = vec![0u8; 256];
//! let res = readpassphrase(c"password: ", &mut buf, Flags::empty());
//! // match_something_on(res);
//! buf.zeroize();
//!
//! let mut pass = readpassphrase_owned(c"password: ", buf, Flags::empty()).unwrap();
//! // do_something_with(&pass);
//! pass.zeroize();
//! ```
//!
//! ## Zeroizing memory
//! This crate works well with the [`::zeroize`] crate. For example, [`::zeroize::Zeroizing`] may
//! be used to zero buffer contents regardless of a function’s control flow:
//! ```no_run
//! # use readpassphrase_3::{Error, Flags, PASSWORD_LEN, getpass, readpassphrase};
//! use zeroize::Zeroizing;
//! # fn main() -> Result<(), Error> {
//! let mut buf = Zeroizing::new(vec![0u8; PASSWORD_LEN]);
//! let pass = readpassphrase(c"pass: ", &mut buf, Flags::REQUIRE_TTY)?;
//! // do_something_that_can_fail_with(pass)?;
//!
//! // Or alternatively:
//! let pass = Zeroizing::new(getpass(c"pass: ")?);
//! // do_something_that_can_fail_with(&pass)?;
//! # Ok(())
//! # }
//! ```
//!
//! If this crate’s `zeroize` feature is enabled, then its [`Zeroize`] will be replaced by a
//! re-export of the upstream [`zeroize::Zeroize`].
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
//! If you need a dynamic prompt, look at [`CString`](std::ffi::CString).
//!
//! # Windows Limitations
//! The Windows implementation of `readpassphrase(3)` that we are using does not yet support UTF-8
//! in prompts; they must be ASCII. It also does not yet support flags, and always behaves as
//! though called with [`Flags::empty()`].
//!
//! [0]: https://man.openbsd.org/readpassphrase

use std::{ffi::CStr, fmt::Display, io, mem, str::Utf8Error};

#[cfg(all(not(docsrs), feature = "zeroize"))]
pub use ::zeroize::Zeroize;
use bitflags::bitflags;
#[cfg(any(docsrs, not(feature = "zeroize")))]
pub use our_zeroize::Zeroize;

/// Size of buffer used in [`getpass`].
///
/// Because `readpassphrase(3)` null-terminates its string, the actual maximum password length for
/// [`getpass`] is 255.
pub const PASSWORD_LEN: usize = 256;

bitflags! {
    /// Flags for controlling readpassphrase.
    ///
    /// The default flag `ECHO_OFF` is not represented here because `bitflags` [recommends against
    /// zero-bit flags][0]; it may be specified as either [`Flags::empty()`] or
    /// [`Flags::default()`].
    ///
    /// Note that the Windows `readpassphrase(3)` implementation always acts like it has been
    /// passed `ECHO_OFF`, i.e., the flags are ignored.
    ///
    /// [0]: https://docs.rs/bitflags/latest/bitflags/#zero-bit-flags
    #[derive(Default)]
    pub struct Flags: i32 {
        /// Leave echo on.
        const ECHO_ON     = 0x01;
        /// Fail if there is no tty.
        const REQUIRE_TTY = 0x02;
        /// Force input to lower case.
        const FORCELOWER  = 0x04;
        /// Force input to upper case.
        const FORCEUPPER  = 0x08;
        /// Strip the high bit from input.
        const SEVENBIT    = 0x10;
        /// Read from stdin, not `/dev/tty`.
        const STDIN       = 0x20;
    }
}

#[deprecated(since = "0.8.0", note = "Use Flags instead")]
pub type RppFlags = Flags;

/// Errors that can occur in readpassphrase.
#[derive(Debug)]
pub enum Error {
    /// `readpassphrase(3)` itself encountered an error.
    Io(io::Error),
    /// The entered password was not UTF-8.
    Utf8(Utf8Error),
}

/// Reads a passphrase using `readpassphrase(3)`, returning a [`&str`](str).
///
/// This function reads a password of up to `buf.len() - 1` bytes into `buf`. If the entered
/// password is longer, it is truncated to the maximum length. If `readpasspharse(3)` itself fails,
/// or if the entered password is not valid UTF-8, then [`Error`] is returned.
///
/// # Security
/// The passed buffer might contain sensitive data, even if this function returns an error.
/// Therefore it should be zeroed as soon as possible. This can be achieved, for example, with
/// [`::zeroize::Zeroizing`]:
/// ```no_run
/// # use readpassphrase_3::{PASSWORD_LEN, Error, Flags, readpassphrase};
/// use zeroize::Zeroizing;
/// # fn main() -> Result<(), Error> {
/// let mut buf = Zeroizing::new(vec![0u8; PASSWORD_LEN]);
/// let pass = readpassphrase(c"Pass: ", &mut buf, Flags::default())?;
/// # Ok(())
/// # }
/// ```
pub fn readpassphrase<'a>(
    prompt: &CStr,
    buf: &'a mut [u8],
    flags: Flags,
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
    Ok(CStr::from_bytes_until_nul(buf).unwrap().to_str()?)
}

/// Reads a passphrase using `readpassphrase(3)`, returning a [`String`].
///
/// Internally, this function uses a buffer of [`PASSWORD_LEN`] bytes, allowing for passwords up to
/// `PASSWORD_LEN - 1` characters (accounting for the C null terminator.) If the entered passphrase
/// is longer, it will be truncated to the maximum length.
///
/// # Security
/// The returned `String` is owned by the caller, and therefore it is the caller’s responsibility
/// to clear it when you are done with it:
/// ```no_run
/// # use readpassphrase_3::{Error, Zeroize, getpass};
/// # fn main() -> Result<(), Error> {
/// let mut pass = getpass(c"Pass: ")?;
/// _ = pass;
/// pass.zeroize();
/// # Ok(())
/// # }
/// ```
pub fn getpass(prompt: &CStr) -> Result<String, Error> {
    Ok(readpassphrase_owned(
        prompt,
        vec![0u8; PASSWORD_LEN],
        Flags::empty(),
    )?)
}

/// An error from [`readpassphrase_owned`].
///
/// This wraps [`Error`] but also contains the passed buffer, accessible via [`OwnedError::take`].
/// If [`take`](OwnedError::take) is not called, the buffer is automatically zeroed on drop.
#[derive(Debug)]
pub struct OwnedError(Error, Option<Vec<u8>>);

/// Reads a passphrase using `readpassphrase(3)`, returning a [`String`] reusing `buf`’s memory.
///
/// This function reads a passphrase of up to `buf.capacity() - 1` bytes. If the entered passphrase
/// is longer, it will be truncated.
///
/// The returned [`String`] reuses `buf`’s memory; no copies are made. On error, the original
/// buffer is instead returned via [`OwnedError`] and may be reused. `OwnedError` converts to
/// [`Error`], so the `?` operator may be used with functions that return `Error`.
///
/// **NB**. Sometimes in Rust the capacity of a vector may be larger than you expect; if you need a
/// precise limit on the length of the entered password, either use [`readpassphrase`] or truncate
/// the returned string.
///
/// # Security
/// The returned `String` is owned by the caller, and it is the caller’s responsibility to clear
/// it. This can be done via [`Zeroize`], e.g.:
/// ```no_run
/// # use readpassphrase_3::{
/// #     PASSWORD_LEN,
/// #     Error,
/// #     Flags,
/// #     readpassphrase_owned,
/// # };
/// # use readpassphrase_3::Zeroize;
/// # fn main() -> Result<(), Error> {
/// let buf = vec![0u8; PASSWORD_LEN];
/// let mut pass = readpassphrase_owned(c"Pass: ", buf, Flags::default())?;
/// _ = pass;
/// pass.zeroize();
/// # Ok(())
/// # }
/// ```
pub fn readpassphrase_owned(
    prompt: &CStr,
    mut buf: Vec<u8>,
    flags: Flags,
) -> Result<String, OwnedError> {
    readpassphrase_mut(prompt, &mut buf, flags).map_err(|e| {
        buf.clear();
        OwnedError(e, Some(buf))
    })
}

// Reads a passphrase into `buf`’s maybe-uninitialized capacity and returns it as a `String`
// reusing `buf`’s memory on success. This function serves to make it possible to write
// `readpassphrase_owned` without either pre-initializing the buffer or invoking undefined
// behavior by constructing a maybe-uninitialized slice.
fn readpassphrase_mut(prompt: &CStr, buf: &mut Vec<u8>, flags: Flags) -> Result<String, Error> {
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

impl OwnedError {
    /// Take `buf` out of the error.
    ///
    /// Returns empty [`Vec`] after the first call.
    pub fn take(&mut self) -> Vec<u8> {
        self.1.take().unwrap_or_default()
    }
}

impl Drop for OwnedError {
    fn drop(&mut self) {
        self.1.take().as_mut().map(Zeroize::zeroize);
    }
}

impl From<OwnedError> for Error {
    fn from(mut value: OwnedError) -> Self {
        mem::replace(&mut value.0, Error::Io(io::ErrorKind::Other.into()))
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

impl core::error::Error for OwnedError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&self.0)
    }
}

impl Display for OwnedError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl core::error::Error for Error {
    fn source(&self) -> Option<&(dyn core::error::Error + 'static)> {
        match self {
            Error::Io(e) => Some(e),
            Error::Utf8(e) => Some(e),
        }
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Io(e) => e.fmt(f),
            Error::Utf8(e) => e.fmt(f),
        }
    }
}

#[cfg(any(docsrs, not(feature = "zeroize")))]
mod our_zeroize {
    use std::{arch::asm, mem::MaybeUninit};

    /// A minimal in-crate implementation of a subset of [`zeroize::Zeroize`].
    ///
    /// This provides compile-fenced memory zeroing for [`String`]s and [`Vec`]s without needing to
    /// depend on the `zeroize` crate.
    ///
    /// If the optional `zeroize` feature is enabled, then the trait is replaced with a re-export of
    /// `zeroize::Zeroize` itself.
    pub trait Zeroize {
        fn zeroize(&mut self);
    }

    impl Zeroize for Vec<u8> {
        fn zeroize(&mut self) {
            self.clear();
            self.spare_capacity_mut().fill(MaybeUninit::zeroed());
            compile_fence(self);
        }
    }

    impl Zeroize for String {
        fn zeroize(&mut self) {
            unsafe { self.as_mut_vec() }.zeroize();
        }
    }

    impl Zeroize for [u8] {
        fn zeroize(&mut self) {
            self.fill(0);
            compile_fence(self);
        }
    }

    fn compile_fence(buf: &[u8]) {
        unsafe {
            asm!(
                "/* {ptr} */",
                ptr = in(reg) buf.as_ptr(),
                options(nostack, preserves_flags, readonly)
            );
        }
    }
}

#[deprecated(
    since = "0.8.0",
    note = "use top-level Zeroize or crate zeroize instead"
)]
pub mod zeroize {
    pub use crate::Zeroize;
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

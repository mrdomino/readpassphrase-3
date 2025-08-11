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

//! This library endeavors to expose a thin wrapper around the C [`readpassphrase(3)`][0] function.
//!
//! Three different interfaces are exposed; for most purposes, you will want to use either
//! [`getpass`] (for simple password entry) or [`readpassphrase`] (when you need flags from
//! `readpassphrase(3)` or need more control over the memory.)
//!
//! The [`readpassphrase_owned`] function is a bit more niche; it may be used when you need a
//! [`String`] output but need to pass flags or control the buffer size (vs [`getpass`].)
//!
//! Sensitive data should be zeroed as soon as possible to avoid leaving it visible in the
//! process’s address space.
//!
//! # Usage
//! To read a passphrase from the console:
//! ```no_run
//! # use readpassphrase_3::{getpass, zeroize::Zeroize};
//! let mut pass = getpass(c"password: ").unwrap();
//! // do_something_with(&pass);
//! pass.zeroize();
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
//! # use readpassphrase_3::{Error, RppFlags, readpassphrase_owned};
//! # fn main() -> Result<(), Error> {
//! # let buf = vec![0u8; 1];
//! let pass = readpassphrase_owned(c"pass: ", buf, RppFlags::empty())?;
//! # Ok(())
//! # }
//! ```
//!
//! This crate works well with the [`zeroize`][1] crate; for example, [`zeroize::Zeroizing`][2] may
//! be used to zero buffer contents regardless of a function’s control flow:
//!
//! ```no_run
//! # use readpassphrase_3::{Error, PASSWORD_LEN, RppFlags, readpassphrase};
//! use zeroize::Zeroizing;
//! # fn main() -> Result<(), Error> {
//! let mut buf = Zeroizing::new(vec![0u8; PASSWORD_LEN]);
//! let pass = readpassphrase(c"pass: ", &mut buf, RppFlags::REQUIRE_TTY)?;
//! // do_something_that_can_fail_with(pass)?;
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
//! [1]: https://docs.rs/zeroize/latest/zeroize/
//! [2]: https://docs.rs/zeroize/latest/zeroize/struct.Zeroizing.html

use std::{ffi::CStr, fmt::Display, io, mem, str::Utf8Error};

use bitflags::bitflags;
use zeroize::Zeroize;

#[cfg(all(not(docsrs), feature = "zeroize"))]
pub use zeroize;

/// Size of buffer used in [`getpass`].
///
/// Because `readpassphrase(3)` null-terminates its string, the actual maximum password length for
/// [`getpass`] is 255.
pub const PASSWORD_LEN: usize = 256;

bitflags! {
    /// Flags for controlling readpassphrase.
    ///
    /// The default flag `ECHO_OFF` is not represented here because `bitflags` [recommends against
    /// zero-bit flags][0]; it may be specified as either [`RppFlags::empty()`] or
    /// [`RppFlags::default()`].
    ///
    /// Note that the Windows `readpassphrase(3)` implementation always acts like it has been
    /// passed `ECHO_OFF`, i.e., the flags are ignored.
    ///
    /// [0]: https://docs.rs/bitflags/latest/bitflags/#zero-bit-flags
    #[derive(Default)]
    pub struct RppFlags: i32 {
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

/// Errors that can occur in readpassphrase.
#[derive(Debug)]
pub enum Error {
    /// `readpassphrase(3)` itself encountered an error.
    Io(io::Error),
    /// The entered password was not UTF-8.
    Utf8(Utf8Error),
}

/// Reads a passphrase using `readpassphrase(3)`.
///
/// This function tries to faithfully wrap `readpassphrase(3)` without overhead; the only
/// additional work it does is:
/// 1. It converts from a Rust byte slice to a C pointer/length pair going in.
/// 2. It converts from a C `char *` to a Rust UTF-8 `&str` coming out.
/// 3. It translates errors from `errno` (or string conversion) into [`Result`].
///
/// This function reads a passphrase of up to `buf.len() - 1` bytes. If the entered passphrase is
/// longer, it will be truncated.
///
/// # Security
/// The passed buffer might contain sensitive data even if this function returns an error (for
/// example, if the contents are not valid UTF-8.) Therefore it should be zeroed as soon as
/// possible. This can be achieved, for example, with [`zeroize::Zeroizing`][0]:
/// ```no_run
/// # use readpassphrase_3::{PASSWORD_LEN, Error, RppFlags, readpassphrase};
/// use zeroize::Zeroizing;
/// # fn main() -> Result<(), Error> {
/// let mut buf = Zeroizing::new(vec![0u8; PASSWORD_LEN]);
/// let pass = readpassphrase(c"Pass: ", &mut buf, RppFlags::default())?;
/// # Ok(())
/// # }
/// ```
///
/// [0]: https://docs.rs/zeroize/latest/zeroize/struct.Zeroizing.html
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
    Ok(CStr::from_bytes_until_nul(buf).unwrap().to_str()?)
}

/// Reads a passphrase using `readpassphrase(3)`, returning it as a [`String`].
///
/// Internally, this function uses a buffer of [`PASSWORD_LEN`] bytes, allowing for passwords up to
/// `PASSWORD_LEN - 1` characters (accounting for the C null terminator.) If the entered passphrase
/// is longer, it will be truncated.
///
/// The passed flags are always [`RppFlags::default()`], i.e. `ECHO_OFF`.
///
/// # Security
/// The returned `String` is owned by the caller, and therefore it is the caller’s responsibility
/// to clear it when you are done with it:
/// ```no_run
/// # use readpassphrase_3::{Error, getpass, zeroize::Zeroize};
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
        RppFlags::empty(),
    )?)
}

/// An error from [`readpassphrase_owned`]. Contains the passed buffer.
#[derive(Debug)]
pub struct OwnedError(Error, Option<Vec<u8>>);

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
/// # use readpassphrase_3::{
/// #     PASSWORD_LEN,
/// #     Error,
/// #     RppFlags,
/// #     readpassphrase_owned,
/// #     zeroize::Zeroize,
/// # };
/// # fn main() -> Result<(), Error> {
/// let buf = vec![0u8; PASSWORD_LEN];
/// let mut pass = readpassphrase_owned(c"Pass: ", buf, RppFlags::default())?;
/// _ = pass;
/// pass.zeroize();
/// # Ok(())
/// # }
/// ```
pub fn readpassphrase_owned(
    prompt: &CStr,
    mut buf: Vec<u8>,
    flags: RppFlags,
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

/// Securely zero the memory in `buf`.
///
/// This function zeroes the full capacity of `buf`, erasing any sensitive data in it. It is
/// a simple shim for [`zeroize`] and the latter should be used instead.
///
/// # Usage
/// The following are equivalent:
/// ```no_run
/// # use readpassphrase_3::{explicit_bzero, zeroize::Zeroize};
/// let mut buf = vec![1u8; 1];
/// // 1.
/// explicit_bzero(&mut buf);
/// // 2.
/// buf.zeroize();
/// ```
#[deprecated(since = "0.6.0", note = "use zeroize::Zeroize instead")]
pub fn explicit_bzero(buf: &mut Vec<u8>) {
    buf.zeroize();
}

impl OwnedError {
    /// Take `buf` out of the error.
    ///
    /// Guaranteed to return `Some(buf)` on the first call, `None` afterwards.
    pub fn take(&mut self) -> Option<Vec<u8>> {
        self.1.take()
    }
}

impl Drop for OwnedError {
    fn drop(&mut self) {
        self.1.take().as_deref_mut().map(zeroize::Zeroize::zeroize);
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

/// A minimal in-crate implementation of [`zeroize::Zeroize`][0].
///
/// This provides compile-fenced memory zeroing for [`String`]s and [`Vec`]s without needing to
/// depend on the `zeroize` crate.
///
/// If the optional `zeroize` feature is enabled, then this module is replaced with `zeroize`
/// itself.
///
/// [0]: https://docs.rs/zeroize/latest/zeroize/trait.Zeroize.html
#[cfg(any(docsrs, not(feature = "zeroize")))]
pub mod zeroize {
    use std::{arch::asm, mem::MaybeUninit};

    /// Trait for securely erasing values from memory.
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

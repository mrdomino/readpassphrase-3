// Copyright 2025
//	Steven Dee
//
// Redistribution and use in source and binary forms, with or without modification, are permitted
// provided that the following conditions are met:
//
// Redistributions of source code must retain the above copyright notice, this list of conditions
// and the following disclaimer.
//
// THIS SOFTWARE IS PROVIDED BY STEVEN DEE “AS IS” AND ANY EXPRESS OR IMPLIED WARRANTIES,
// INCLUDING, BUT NOT LIMITED TO, THE IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A
// PARTICULAR PURPOSE ARE DISCLAIMED. IN NO EVENT SHALL STEVEN DEE BE LIABLE FOR ANY DIRECT,
// INDIRECT, INCIDENTAL, SPECIAL, EXEMPLARY, OR CONSEQUENTIAL DAMAGES (INCLUDING, BUT NOT LIMITED
// TO, PROCUREMENT OF SUBSTITUTE GOODS OR SERVICES; LOSS OF USE, DATA, OR PROFITS; OR BUSINESS
// INTERRUPTION) HOWEVER CAUSED AND ON ANY THEORY OF LIABILITY, WHETHER IN CONTRACT, STRICT
// LIABILITY, OR TORT (INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY OUT OF THE USE OF THIS
// SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF SUCH DAMAGE.

//! Lightweight, easy-to-use wrapper around the C [`readpassphrase(3)`][0] function.
//!
//! From the man page:
//! > The `readpassphrase()` function displays a prompt to, and reads in a passphrase from,
//! > `/dev/tty`. If this file is inaccessible and the [`RPP_REQUIRE_TTY`](Flags::REQUIRE_TTY) flag
//! > is not set, `readpassphrase()` displays the prompt on the standard error output and reads
//! > from the standard input.
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
//! [`readpassphrase`] or [`readpassphrase_into`] depending on your ownership requirements:
//! ```no_run
//! let mut buf = vec![0u8; 256];
//! use readpassphrase_3::{Flags, readpassphrase};
//! let pass: &str = readpassphrase(c"Password: ", &mut buf, Flags::default()).unwrap();
//!
//! use readpassphrase_3::readpassphrase_into;
//! let pass: String = readpassphrase_into(c"Pass: ", buf, Flags::FORCELOWER).unwrap();
//! # _ = pass;
//! ```
//!
//! # Security
//! The [`readpassphrase(3)` man page][0] says:
//! > The calling process should zero the passphrase as soon as possible to avoid leaving the
//! > cleartext passphrase visible in the process's address space.
//!
//! It is your job to ensure that this is done with the data you own, i.e.
//! any [`Vec`] passed to [`readpassphrase`] or any [`String`] received from [`getpass`] or
//! [`readpassphrase_into`].
//!
//! This crate ships with a minimal [`Zeroize`] trait that may be used for this purpose:
//! ```no_run
//! # use readpassphrase_3::{Flags, getpass, readpassphrase, readpassphrase_into};
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
//! let mut pass = readpassphrase_into(c"password: ", buf, Flags::empty()).unwrap();
//! // do_something_with(&pass);
//! pass.zeroize();
//! ```
//!
//! ## Zeroizing memory
//! This crate works well with the [`zeroize`] crate. For example, [`zeroize::Zeroizing`] may be
//! used to zero buffer contents regardless of a function’s control flow:
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
//! The prompt strings in this API are <code>&[CStr]</code>, not <code>&[str]</code>.
//! This is because the underlying C function assumes that the prompt is a null-terminated string;
//! were we to take `&str` instead of `&CStr`, we would need to make a copy of the prompt on every
//! call.
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
//! [str]: prim@str "str"

use std::{error, ffi::CStr, fmt, io, mem, str};

use bitflags::bitflags;
#[cfg(any(docsrs, not(feature = "zeroize")))]
pub use our_zeroize::Zeroize;
#[cfg(all(not(docsrs), feature = "zeroize"))]
pub use zeroize::Zeroize;

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

/// Errors that can occur in readpassphrase.
#[derive(Debug)]
pub enum Error {
    /// `readpassphrase(3)` itself encountered an error.
    Io(io::Error),
    /// The entered password was not UTF-8.
    Utf8(str::Utf8Error),
}

/// Reads a passphrase using `readpassphrase(3)`.
///
/// This function returns a <code>&[str]</code> backed by `buf`, representing a password of up to
/// `buf.len() - 1` bytes. Any additional characters and the terminating newline are discarded.
///
/// # Errors
/// Returns [`Err`] if `readpassphrase(3)` itself failed or if the entered password is not UTF-8.
/// The former will be represented by [`Error::Io`] and the latter by [`Error::Utf8`].
///
/// # Security
/// The passed buffer might contain sensitive data, even if this function returns an error.
/// Therefore it should be zeroed as soon as possible. This can be achieved, for example, with
/// [`zeroize::Zeroizing`]:
/// ```no_run
/// # use readpassphrase_3::{PASSWORD_LEN, Error, Flags, readpassphrase};
/// use zeroize::Zeroizing;
/// # fn main() -> Result<(), Error> {
/// let mut buf = Zeroizing::new(vec![0u8; PASSWORD_LEN]);
/// let pass = readpassphrase(c"Pass: ", &mut buf, Flags::default())?;
/// # Ok(())
/// # }
/// ```
/// [str]: prim@str "str"
pub fn readpassphrase<'a>(
    prompt: &CStr,
    buf: &'a mut [u8],
    flags: Flags,
) -> Result<&'a str, Error> {
    let prompt = prompt.as_ptr();
    let buf_ptr = buf.as_mut_ptr().cast();
    let bufsiz = buf.len();
    let flags = flags.bits();
    // SAFETY: `prompt` is a nul-terminated byte sequence, and `buf_ptr` is an allocation of at
    // least `bufsiz` bytes, as guaranteed by `&CStr` and `&mut [u8]` respectively.
    let res = unsafe { ffi::readpassphrase(prompt, buf_ptr, bufsiz, flags) };
    if res.is_null() {
        return Err(io::Error::last_os_error().into());
    }
    Ok(CStr::from_bytes_until_nul(buf).unwrap().to_str()?)
}

/// Reads a passphrase using `readpassphrase(3)`, returning a [`String`].
///
/// Internally, this function uses a buffer of [`PASSWORD_LEN`] bytes, allowing for passwords up to
/// `PASSWORD_LEN - 1` characters (accounting for the C null terminator.) If the entered passphrase
/// is longer, it will be truncated to the maximum length.
///
/// # Errors
/// Returns [`Err`] if `readpassphrase(3)` itself failed or if the entered password is not UTF-8.
/// The former will be represented by [`Error::Io`] and the latter by [`Error::Utf8`].
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
    let buf = Vec::with_capacity(PASSWORD_LEN);
    Ok(readpassphrase_into(prompt, buf, Flags::empty())?)
}

/// An [`Error`] from [`readpassphrase_into`] containing the passed buffer.
///
/// The buffer is accessible via [`IntoError::into_bytes`][0], and the `Error` via
/// [`IntoError::error`].
///
/// If [`into_bytes`][0] is not called, the buffer is automatically zeroed on drop.
///
/// This struct is also exported as [`OwnedError`]. That name is deprecated; please transition to
/// using `IntoError` instead.
///
/// [0]: IntoError::into_bytes
#[derive(Debug)]
pub struct IntoError(Error, Option<Vec<u8>>);

/// Reads a passphrase using `readpassphrase(3)`, returning `buf` as a [`String`].
///
/// This function reads a passphrase of up to `buf.capacity() - 1` bytes. If the entered passphrase
/// is longer, it will be truncated.
///
/// The returned [`String`] reuses `buf`’s memory; no copies are made.
///
/// **NB**. Sometimes in Rust the capacity of a vector may be larger than you expect; if you need a
/// precise limit on the length of the entered password, either use [`readpassphrase`] or truncate
/// the returned string.
///
/// # Errors
/// Returns [`Err`] if `readpassphrase(3)` itself failed or if the entered password is not UTF-8.
/// The former will be represented by [`Error::Io`] and the latter by [`Error::Utf8`]. The vector
/// you moved in is also included.
///
/// See the docs for [`IntoError`] for more details on what you can do with this error.
///
/// # Security
/// The returned `String` is owned by the caller, and it is the caller’s responsibility to clear
/// it. This can be done via [`Zeroize`], e.g.:
/// ```no_run
/// # use readpassphrase_3::{
/// #     PASSWORD_LEN,
/// #     Error,
/// #     Flags,
/// #     readpassphrase_into,
/// # };
/// # use readpassphrase_3::Zeroize;
/// # fn main() -> Result<(), Error> {
/// let buf = vec![0u8; PASSWORD_LEN];
/// let mut pass = readpassphrase_into(c"Pass: ", buf, Flags::default())?;
/// _ = pass;
/// pass.zeroize();
/// # Ok(())
/// # }
/// ```
pub fn readpassphrase_into(
    prompt: &CStr,
    mut buf: Vec<u8>,
    flags: Flags,
) -> Result<String, IntoError> {
    let prompt = prompt.as_ptr();
    let buf_ptr = buf.as_mut_ptr().cast();
    let bufsiz = buf.capacity();
    let flags = flags.bits();
    // SAFETY: `prompt` from `&CStr` as above. `buf_ptr` points to an allocation of `bufsiz` bytes.
    let res = unsafe { ffi::readpassphrase(prompt, buf_ptr, bufsiz, flags) };
    if res.is_null() {
        buf.clear();
        return Err(IntoError(io::Error::last_os_error().into(), Some(buf)));
    }
    let nul_pos = (0..bufsiz as isize)
        // SAFETY: `i` is within `bufsiz`, which is the size of `buf`’s allocation;
        // `ffi::readpassphrase` initialized `buf` up through a zero byte. We scan `buf` in order;
        // the zero byte we find is at or before the end of the initialized portion.
        .position(|i| unsafe { *buf_ptr.offset(i) == 0 })
        .unwrap();
    // SAFETY: `buf` is initialized at least up to `nul_pos`.
    unsafe { buf.set_len(nul_pos) };
    String::from_utf8(buf).map_err(|err| {
        let res = err.utf8_error();
        IntoError(res.into(), Some(err.into_bytes()))
    })
}

#[deprecated(since = "0.10.0", note = "please use `IntoError`")]
pub use IntoError as OwnedError;

/// Deprecated alias for [`readpassphrase_into`].
#[deprecated(since = "0.10.0", note = "please use `readpassphrase_into`")]
pub fn readpassphrase_owned(
    prompt: &CStr,
    buf: Vec<u8>,
    flags: Flags,
) -> Result<String, IntoError> {
    readpassphrase_into(prompt, buf, flags)
}

impl IntoError {
    /// Return the [`Error`] corresponding to this.
    pub fn error(&self) -> &Error {
        &self.0
    }

    /// Returns the buffer that was passed to [`readpassphrase_into`].
    ///
    /// # Panics
    /// Panics if [`IntoError::take`] was called before this.
    pub fn into_bytes(mut self) -> Vec<u8> {
        self.1.take().unwrap()
    }

    /// Returns the buffer that was passed to [`readpassphrase_into`].
    ///
    /// If called multiple times, returns [`Vec::new`].
    #[deprecated(since = "0.10.0", note = "please use `into_bytes` instead")]
    pub fn take(&mut self) -> Vec<u8> {
        self.1.take().unwrap_or_default()
    }
}

impl error::Error for IntoError {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        Some(&self.0)
    }
}

impl fmt::Display for IntoError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl Drop for IntoError {
    fn drop(&mut self) {
        self.1.take().as_mut().map(Zeroize::zeroize);
    }
}

impl From<IntoError> for Error {
    fn from(mut value: IntoError) -> Self {
        mem::replace(&mut value.0, Error::Io(io::ErrorKind::Other.into()))
    }
}

impl From<io::Error> for Error {
    fn from(value: io::Error) -> Self {
        Error::Io(value)
    }
}

impl From<str::Utf8Error> for Error {
    fn from(value: str::Utf8Error) -> Self {
        Error::Utf8(value)
    }
}

impl error::Error for Error {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        Some(match self {
            Error::Io(e) => e,
            Error::Utf8(e) => e,
        })
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
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
            let buf = self.spare_capacity_mut();
            buf.fill(MaybeUninit::zeroed());
            compile_fence(buf);
        }
    }

    impl Zeroize for String {
        fn zeroize(&mut self) {
            // SAFETY: we clear the string.
            unsafe { self.as_mut_vec() }.zeroize();
        }
    }

    impl Zeroize for [u8] {
        fn zeroize(&mut self) {
            self.fill(0);
            compile_fence(self);
        }
    }

    fn compile_fence<T>(buf: &[T]) {
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

    extern "C" {
        pub(crate) fn readpassphrase(
            prompt: *const c_char,
            buf: *mut c_char,
            bufsiz: usize,
            flags: c_int,
        ) -> *mut c_char;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty() {
        let err = readpassphrase_into(c"pass", Vec::new(), Flags::empty()).unwrap_err();
        let Error::Io(err) = err.into() else {
            panic!();
        };
        #[cfg(not(windows))]
        assert_eq!(io::ErrorKind::InvalidInput, err.kind());
        #[cfg(windows)]
        {
            _ = err
        };

        let mut buf = Vec::new();
        let err = readpassphrase(c"pass", &mut buf, Flags::empty()).unwrap_err();
        let Error::Io(err) = err else {
            panic!();
        };
        #[cfg(not(windows))]
        assert_eq!(io::ErrorKind::InvalidInput, err.kind());
        #[cfg(windows)]
        {
            _ = err
        };
    }

    #[test]
    fn test_zeroize() {
        let mut buf = "test".to_string();
        buf.zeroize();
        unsafe { buf.as_mut_vec().set_len(4) };
        assert_eq!("\0\0\0\0", &buf);
        let mut buf = vec![1u8; 15];
        unsafe { buf.set_len(0) };
        let x = buf.spare_capacity_mut()[0];
        assert_eq!(unsafe { x.assume_init() }, 1);
        buf.zeroize();
        unsafe { buf.set_len(15) };
        assert_eq!(vec![0u8; 15], buf);
        let mut buf = vec![1u8; 2];
        unsafe { buf.set_len(1) };
        let slice = &mut *buf;
        slice.zeroize();
        unsafe { buf.set_len(2) };
        assert_eq!(vec![0u8, 1], buf);
    }
}

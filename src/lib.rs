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
//! [`readpassphrase`] or [`readpassphrase_owned`] depending on your ownership requirements:
//! ```no_run
//! let mut buf = vec![0u8; 256];
//! use readpassphrase_3::{Flags, readpassphrase};
//! let pass: &str = readpassphrase(c"Password: ", &mut buf, Flags::default()).unwrap();
//!
//! use readpassphrase_3::readpassphrase_owned;
//! let pass: String = readpassphrase_owned(c"Pass: ", buf, Flags::FORCELOWER).unwrap();
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

use std::{error, ffi::CStr, fmt, io, mem, slice, str::Utf8Error};

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
    Utf8(Utf8Error),
}

/// Reads a passphrase using `readpassphrase(3)`, returning a [`&str`](str).
///
/// This function reads a password of up to `buf.len() - 1` bytes into `buf`. If the entered
/// password is longer, it is truncated to the maximum length. If `readpassphrase(3)` itself fails,
/// or if the entered password is not valid UTF-8, then [`Error`] is returned.
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
    Ok(readpassphrase_owned(prompt, buf, Flags::empty())?)
}

/// An [`Error`] from [`readpassphrase_owned`] containing the passed buffer.
///
/// The buffer is accessible via [`OwnedError::take`]. If [`take`](OwnedError::take) is not called,
/// the buffer is automatically zeroed on drop.
#[derive(Debug)]
pub struct OwnedError(Error, Option<Vec<u8>>);

/// Reads a passphrase using `readpassphrase(3)`, returning `buf` as a [`String`].
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

// Reads a passphrase into `buf`’s full capacity and returns it as a `String` reusing `buf`’s
// memory on success. This function serves to make it possible to write `readpassphrase_owned`
// without either pre-initializing the buffer or invoking undefined behavior by constructing a
// potentially uninitialized slice.
fn readpassphrase_mut(prompt: &CStr, buf: &mut Vec<u8>, flags: Flags) -> Result<String, Error> {
    // If we could construct a `&[u8]` out of potentially uninitialized memory, then this whole
    // function could just be:
    // ```
    // let buf_slice = unsafe { slice::from_raw_parts_mut(buf.as_mut_ptr(), buf.capacity()) };
    // let res = readpassphrase(prompt, buf_slice, flags)?;
    // unsafe {
    //     buf.set_len(res.len());
    // }
    // Ok(unsafe { String::from_utf8_unchecked(mem::take(buf)) })
    // ```
    let prompt = prompt.as_ptr();
    let buf_ptr: *mut mem::MaybeUninit<u8> = buf.as_mut_ptr().cast();
    let bufsiz = buf.capacity();
    let flags = flags.bits();
    // SAFETY: as in `crate::readpassphrase`.
    let res = unsafe { ffi::readpassphrase(prompt, buf_ptr.cast(), bufsiz, flags) };
    if res.is_null() {
        return Err(io::Error::last_os_error().into());
    }

    // SAFETY: `buf` will not be mutated from here on.
    let buf_uninit = unsafe { slice::from_raw_parts(buf_ptr, bufsiz) };
    let nul_pos = buf_uninit
        .iter()
        .position(|&b| unsafe {
            // We assume that `readpassphrase(3)` either returns null or initializes `buf`
            // to a sequence of bytes ending in a zero byte. This assumption is unchecked.
            b.assume_init() == 0
        })
        .unwrap();
    // SAFETY: just confirmed that `buf` has its first nul byte at `nul_pos < bufsiz`.
    let res = unsafe {
        let bytes = slice::from_raw_parts(buf_ptr.cast(), nul_pos + 1);
        CStr::from_bytes_with_nul_unchecked(bytes)
    }
    .to_str()?;
    // SAFETY: `buf` is initialized up to `res.len() == nul_pos < bufsiz == buf.capacity()`.
    unsafe {
        buf.set_len(res.len());
    }
    let buf = mem::take(buf);
    // SAFETY: confirmed via `CStr::to_str`.
    Ok(unsafe { String::from_utf8_unchecked(buf) })
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

impl error::Error for OwnedError {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        Some(&self.0)
    }
}

impl fmt::Display for OwnedError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
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

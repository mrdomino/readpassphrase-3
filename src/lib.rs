// Copyright 2025 Steven Dee.
//
// This project is dual licensed under the MIT and Apache 2.0 licenses. See
// the LICENSE file in the project root for details.
//
// The readpassphrase source and header are copyright 2000-2002, 2007, 2010
// Todd C. Miller.

use std::{
    ffi::{CStr, FromBytesUntilNulError},
    fmt::Display,
    io, mem,
    str::Utf8Error,
};

use bitflags::bitflags;
#[cfg(feature = "zeroize")]
use zeroize::Zeroizing;

pub const PASSWORD_LEN: usize = 256;

bitflags! {
    /// Flags for controlling readpassphrase
    pub struct RppFlags: i32 {
        /// Furn off echo (default)
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

#[derive(Debug)]
pub enum Error {
    Io(io::Error),
    Utf8(Utf8Error),
    CStr(FromBytesUntilNulError),
}

/// Reads a passphrase using `readpassphrase(3)`, returning it as a `String`.
/// Internally uses a buffer of `PASSWORD_LEN` bytes, allowing for passwords
/// up to `PASSWORD_LEN - 1` characters (including the null terminator.)
///
/// # Security
/// The returned `String` is not cleared on success; it is the caller’s
/// responsibility to do so, e.g.:
///
/// ```no_run
/// # use readpassphrase_3::{Error, RppFlags, readpassphrase};
/// # use zeroize::Zeroizing;
/// # fn main() -> Result<(), Error> {
/// let pass = Zeroizing::new(readpassphrase(c"Pass: ", RppFlags::default())?);
/// # Ok(())
/// # }
/// ```
pub fn readpassphrase(prompt: &CStr, flags: RppFlags) -> Result<String, Error> {
    readpassphrase_buf(prompt, vec![0u8; PASSWORD_LEN], flags)
}

/// Reads a passphrase using `readpassphrase(3)` into the passed buffer.
/// Returns a `String` consisting of the same memory from the buffer. If
/// the `zeroize` feature is enabled (which it is by default), memory is
/// cleared on errors.
///
/// # Security
/// The returned `String` is not cleared on success; it is the caller’s
/// responsibility to do so, e.g.:
///
/// ```no_run
/// # use readpassphrase_3::{PASSWORD_LEN, Error, RppFlags, readpassphrase_buf};
/// # use zeroize::Zeroizing;
/// # fn main() -> Result<(), Error> {
/// let buf = vec![0u8; PASSWORD_LEN];
/// let pass = Zeroizing::new(readpassphrase_buf(c"Pass: ", buf, RppFlags::default())?);
/// # Ok(())
/// # }
/// ```
pub fn readpassphrase_buf(
    prompt: &CStr,
    #[allow(unused_mut)] mut buf: Vec<u8>,
    flags: RppFlags,
) -> Result<String, Error> {
    #[cfg(feature = "zeroize")]
    let mut buf = Zeroizing::new(buf);
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
    let nul_pos = buf
        .iter()
        .position(|&b| b == 0)
        .ok_or(io::Error::from(io::ErrorKind::InvalidData))?;
    buf.truncate(nul_pos);
    let _ = str::from_utf8(&buf)?;
    Ok(unsafe { String::from_utf8_unchecked(mem::take(&mut buf)) })
}

/// Reads a passphrase using `readpassphrase(3)` into the passed buffer.
/// Returns a string slice from that buffer.
///
/// # Security
/// Does not zero memory; this should be done out of band, for example by
/// using `Zeroizing<Vec<u8>>`:
/// ```no_run
/// # use readpassphrase_3::{PASSWORD_LEN, Error, RppFlags, readpassphrase_inplace};
/// # use zeroize::Zeroizing;
/// # fn main() -> Result<(), Error> {
/// let mut buf = Zeroizing::new(vec![0u8; PASSWORD_LEN]);
/// let pass = readpassphrase_inplace(c"Pass: ", &mut buf, RppFlags::default())?;
/// # Ok(())
/// # }
/// ```
pub fn readpassphrase_inplace<'a>(
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
    let res = CStr::from_bytes_until_nul(buf)?;
    Ok(res.to_str()?)
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

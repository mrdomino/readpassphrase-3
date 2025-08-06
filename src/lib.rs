// Copyright 2025 Steven Dee.
//
// This project is dual licensed under the MIT and Apache 2.0 licenses. See the
// LICENSE file in the project root for details.
//
// The readpassphrase source and header are copyright 2000-2002, 2007, 2010 Todd
// C. Miller.

use std::{ffi::CStr, io, mem, str::Utf8Error};

use bitflags::bitflags;
use thiserror::Error;
use zeroize::Zeroizing;

const PASSWORD_LEN: usize = 256;

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

#[derive(Error, Debug)]
pub enum Error {
    #[error(transparent)]
    IoError(#[from] io::Error),
    #[error(transparent)]
    Utf8Error(#[from] Utf8Error),
}

pub fn readpassphrase(prompt: &CStr, flags: RppFlags) -> Result<String, Error> {
    readpassphrase_buf(prompt, vec![0u8; PASSWORD_LEN], flags)
}

pub fn readpassphrase_buf(prompt: &CStr, buf: Vec<u8>, flags: RppFlags) -> Result<String, Error> {
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

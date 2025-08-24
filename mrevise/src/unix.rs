use core::{
    ffi::{c_int, c_void},
    ptr::null_mut,
};

use std::error::Error;

use super::{last_error, AllocFlags, Prot, ProtectResult};

const PROT_NONE: c_int = 0x0;
const PROT_READ: c_int = 0x1;
const PROT_WRITE: c_int = 0x2;
const PROT_EXEC: c_int = 0x4;

const MAP_PRIVATE: c_int = 0x02;
#[cfg(target_os = "linux")]
const MAP_ANON: c_int = 0x20;
#[cfg(not(target_os = "linux"))]
const MAP_ANON: c_int = 0x1000;

unsafe extern "C" {
    fn mprotect(addr: *mut c_void, len: usize, prot: c_int) -> c_int;

    // TODO: Pretty sure offset data type is wrong on 32-bit Linux.
    fn mmap(
        addr: *mut c_void,
        len: usize,
        prot: c_int,
        flags: c_int,
        fd: c_int,
        offset: i64,
    ) -> *mut c_void;
}

pub fn protect<P>(addr: *mut P, size: usize, prot: Prot) -> Result<ProtectResult, Box<dyn Error>> {
    let c_prot: c_int = match prot {
        Prot::None => PROT_NONE,
        Prot::Read => PROT_READ,
        Prot::ReadWrite => PROT_READ | PROT_WRITE,
        Prot::ReadWriteExecute => PROT_READ | PROT_WRITE | PROT_EXEC,
        Prot::Custom(v) => v as c_int,
    };

    let result = unsafe { mprotect(addr.cast(), size, c_prot) };
    if result != 0 {
        return Err(last_error("mprotect failed"))?;
    }

    Ok(ProtectResult { old: None })
}

pub fn alloc<P>(
    addr: Option<*mut P>,
    length: usize,
    prot: Prot,
    flags: AllocFlags,
) -> Result<*mut P, Box<dyn Error>> {
    let pointer = match addr {
        Some(p) => p,
        None => null_mut(),
    };

    let unix_prot: c_int = prot_to_unix_const(prot);

    let unix_flags = match flags {
        AllocFlags::Default => MAP_PRIVATE | MAP_ANON,
        AllocFlags::Custom(v) => v as c_int,
    };

    let result = unsafe { mmap(pointer.cast(), length, unix_prot, unix_flags, -1, 0) };
    if result.is_null() {
        return Err(last_error("mmap failed"))?;
    }

    Ok(result as *mut P)
}

fn prot_to_unix_const(prot: Prot) -> c_int {
    match prot {
        Prot::None => PROT_NONE,
        Prot::Read => PROT_READ,
        Prot::ReadWrite => PROT_READ | PROT_WRITE,
        Prot::ReadWriteExecute => PROT_READ | PROT_WRITE | PROT_EXEC,
        Prot::Custom(v) => v as c_int,
    }
}

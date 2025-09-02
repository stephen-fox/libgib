// PAGE_PROTECTION_FLAGS = u32

use core::{
    ffi::c_void,
    ptr::null_mut,
};

use std::error::Error;

use super::{last_error, AllocFlags, Prot, ProtectResult};

const MEM_COMMIT: u32 = 0x00001000;
const MEM_RESERVE: u32 = 0x00002000;

#[link(name = "kernel32")]
unsafe extern "system" {
    fn VirtualProtect(
        lpaddress: *mut c_void,
        dwsize: usize,
        flnewprotect: u32,
        lpfloldprotect: *mut u32,
    ) -> bool;

    fn VirtualAlloc(
        lpaddress: *mut c_void,
        dwsize: usize,
        flallocationtype: u32,
        flprotect: u32,
    ) -> *mut c_void;
}

pub fn protect<P>(addr: *mut P, size: usize, prot: Prot) -> Result<ProtectResult, Box<dyn Error>> {
    let windows_prot: u32 = prot_to_windows_const(prot);

    let mut old_protect: u32 = 0;

    let result = unsafe { VirtualProtect(addr.cast(), size, windows_prot, &mut old_protect) };
    if !result {
        return Err(last_error("VirtualProtect failed"))?;
    }

    Ok(ProtectResult {
        old: Some(old_protect),
    })
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

    let windows_prot: u32 = prot_to_windows_const(prot);

    let alloc_type = match flags {
        AllocFlags::Default => MEM_COMMIT | MEM_RESERVE,
        AllocFlags::Custom(v) => v as u32,
    };

    let result = unsafe { VirtualAlloc(pointer.cast(), length, alloc_type, windows_prot) };
    if result.is_null() {
        return Err(last_error("VirtualAlloc failed"))?;
    }

    Ok(result as *mut P)
}

fn prot_to_windows_const(prot: Prot) -> u32 {
    // https://learn.microsoft.com/en-us/windows/win32/Memory/memory-protection-constants
    match prot {
        Prot::None => 0x01,             // PAGE_NOACCESS
        Prot::Read => 0x02,             // PAGE_READONLY
        Prot::ReadWrite => 0x04,        // PAGE_READWRITE
        Prot::ReadWriteExecute => 0x40, // PAGE_EXECUTE_READWRITE
        Prot::Custom(v) => v,
    }
}

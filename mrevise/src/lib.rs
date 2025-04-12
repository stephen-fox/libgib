//! Module for memory manipulation and searching logic

use std::error::Error;

/// Attempts to find a matching pattern anywhere between the start and
/// end offsets
///
/// ## Safety
///
/// Reading program memory is *NOT* safe but its required for pattern matching
///
/// ## Arguments
/// * start_offset - The address to start matching from
/// * end_offset   - The address to stop matching at
/// * mask         - The mask to use when matching opcodes
/// * bytes        - The bytes to match against
pub unsafe fn find_pattern(
    start_offset: usize,
    end_offset: usize,
    mask: &'static str,
    bytes: &'static [u8],
) -> Option<*const u8> {
    // Iterate between the offsets
    (start_offset..=end_offset)
        // Cast the address to a pointer type
        .map(|addr| addr as *const u8)
        // Compare the mask at the provided address
        .find(|addr| unsafe { compare_mask(*addr, mask, bytes) })
}

/// Compares the opcodes after the provided address using the provided
/// opcode and pattern
///
/// ## Safety
///
/// Reading program memory is *NOT* safe but its required for pattern matching
///
/// ## Arguments
/// * addr  - The address to start matching from
/// * mask  - The mask to use when matching opcodes
/// * bytes - The bytes to match against
pub unsafe fn compare_mask(addr: *const u8, mask: &'static str, bytes: &'static [u8]) -> bool {
    mask.chars()
        .enumerate()
        // Merge the iterator with the opcodes for matching
        .zip(bytes.iter().copied())
        // Compare the mask and memory at the address with the op codes
        .all(|((offset, mask), op)| mask == '?' || unsafe { *addr.add(offset) } == op)
}

pub struct MemAttrs {
    pub length: usize,
    pub align_to: Option<usize>,
    pub prot_during: Option<Prot>,
    pub prot_after: Option<Prot>,
    pub try_restore_orig_prot: bool,
}

pub enum Prot {
    None,
    Read,
    ReadWrite,
    ReadWriteExecute,
    Custom(u32),
}

impl std::fmt::Display for Prot {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Prot::None => "none",
            Prot::Read => "read",
            Prot::ReadWrite => "read-write",
            Prot::ReadWriteExecute => "read-write-execute",
            Prot::Custom(v) => &format!("custom ({v})"),
        };

        write!(f, "{s}")
    }
}

/// Attempts to apply virtual protect READ/WRITE access
/// over the memory at the provided address for the length
/// provided. Restores the original flags after the action
/// is complete
///
/// ## Safety
///
/// This function acquires the proper write permissions over
/// `addr` for the required `length` but it is unsound if
/// memory past `length` is accessed
///
/// ## Arguments
/// TODO
#[inline]
pub unsafe fn use_memory<F, P>(
    pointer: *const P,
    attrs: MemAttrs,
    action: F,
) -> Result<(), Box<dyn Error>>
where
    F: FnOnce(*mut P),
{
    let mut protect_ptr: *mut P = pointer.cast_mut();
    let mut chunk_size = attrs.length;

    if let Some(align_bits) = attrs.align_to {
        let adjustment = align_to(protect_ptr, align_bits, attrs.length);

        protect_ptr = adjustment.new_ptr;
        chunk_size = adjustment.size_to_modify;
    }

    let mut orig_prot: Option<Prot> = None;

    if let Some(prot_during) = attrs.prot_during {
        match protect(protect_ptr, chunk_size, prot_during, None) {
            Ok(i) => {
                if let Some(old) = i.old {
                    orig_prot = Some(Prot::Custom(old));
                }
            }
            Err(err) => return Err(format!(
                "failed to protect memory region 0x{:x?} (orig: 0x{:x?}) length 0x{:x?} (orig: 0x{:x?}) - {}",
                protect_ptr.addr(),
                pointer.addr(),
                chunk_size,
                attrs.length,
                err
            ))?,
        };
    }

    action(pointer.cast_mut());

    let prot_after = match attrs.prot_after {
        Some(p) => p,
        None => return Ok(()),
    };

    let mut final_prot = prot_after;
    if attrs.try_restore_orig_prot && orig_prot.is_some() {
        final_prot = orig_prot.unwrap();
    }

    match protect(protect_ptr, chunk_size, final_prot, None) {
        Ok(_) => Ok(()),
        Err(err) => return Err(format!(
            "failed to restore memory region protection at 0x{:x?} (orig: 0x{:x?}) length 0x{:x?} (orig: 0x{:x?}) - {}",
            protect_ptr.addr(),
            pointer.addr(),
            chunk_size,
            attrs.length,
            err
        ))?,
    }
}

pub fn protect<P>(
    pointer: *mut P,
    size: usize,
    prot: Prot,
    align_with: Option<usize>,
) -> Result<ProtectResult, Box<dyn Error>> {
    let mut target_ptr = pointer;
    let mut chunk_size = size;

    if let Some(align_bits) = align_with {
        let adjustment = align_to(target_ptr.cast(), align_bits, chunk_size);

        target_ptr = adjustment.new_ptr;
        chunk_size = adjustment.size_to_modify;
    }

    #[cfg(unix)]
    let result = unix::protect(target_ptr, chunk_size, prot);

    #[cfg(target_os = "windows")]
    let result = windows::protect(target_ptr, chunk_size, prot);

    result
}

pub struct ProtectResult {
    pub old: Option<u32>,
}

pub enum AllocFlags {
    Default,
    Custom(u64),
}

pub fn alloc<P>(
    addr: Option<*mut P>,
    length: usize,
    prot: Prot,
    flags: AllocFlags,
) -> Result<*mut P, Box<dyn Error>> {
    #[cfg(unix)]
    let result = unix::alloc(addr, length, prot, flags);

    #[cfg(windows)]
    let result = windows::alloc(addr, length, prot, flags);

    result
}

struct AlignToOutput<P> {
    new_ptr: *mut P,
    size_to_modify: usize,
}

fn align_to<P>(pointer: *mut P, bits: usize, chunk_size: usize) -> AlignToOutput<P> {
    let current_addr = pointer.addr();

    let new_addr = current_addr & !(bits - 1);

    let diff: usize;

    match new_addr {
        new_addr if current_addr == new_addr => {
            return AlignToOutput {
                new_ptr: pointer,
                size_to_modify: chunk_size,
            }
        }
        new_addr if current_addr > new_addr => diff = current_addr - new_addr,
        _ => diff = new_addr - current_addr,
    };

    AlignToOutput {
        new_ptr: new_addr as *mut P,
        size_to_modify: diff + chunk_size,
    }
}

#[cfg(unix)]
mod unix {
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

    pub fn protect<P>(
        addr: *mut P,
        size: usize,
        prot: Prot,
    ) -> Result<ProtectResult, Box<dyn Error>> {
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
}

#[cfg(target_os = "windows")]
mod windows {
    // PAGE_PROTECTION_FLAGS = u32

    use core::{
        ffi::{c_int, c_void},
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

    pub fn protect<P>(
        addr: *mut P,
        size: usize,
        prot: Prot,
    ) -> Result<ProtectResult, Box<dyn Error>> {
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
}

fn last_error(prefix: &str) -> std::io::Error {
    std::io::Error::new(
        std::io::ErrorKind::Other,
        format!("{prefix} - {err}", err = std::io::Error::last_os_error()),
    )
}

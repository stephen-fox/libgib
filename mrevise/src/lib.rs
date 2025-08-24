//! Module for memory manipulation and searching logic

use std::error::Error;

#[cfg(unix)]
pub mod unix;

#[cfg(target_os = "windows")]
pub mod windows;

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

fn last_error(prefix: &str) -> std::io::Error {
    std::io::Error::new(
        std::io::ErrorKind::Other,
        format!("{prefix} - {err}", err = std::io::Error::last_os_error()),
    )
}

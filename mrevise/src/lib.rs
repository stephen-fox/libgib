use std::error::Error;

#[cfg(unix)]
pub mod unix;

#[cfg(target_os = "windows")]
pub mod windows;

/// find_pattern attempts to find a matching pattern anywhere between
/// the start and end offsets.
///
/// ## Safety
///
/// This function is unsafe because it interacts with memory that may be
/// owned by other code or memory that is being operated on concurrently
/// by another thread.
///
/// ## Arguments
///
/// * `start_offset` - The address to start matching from.
/// * `end_offset`   - The address to stop matching at.
/// * `mask`         - The mask to use when matching opcodes.
/// * `bytes`        - The bytes to match against.
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

/// compare_mask compares the bytes after the provided address using
/// the provided pattern.
///
/// ## Safety
///
/// This function is unsafe because it interacts with memory that may be
/// owned by other code or memory that is being operated on concurrently
/// by another thread.
///
/// ## Arguments
///
/// * `addr`  - The address to start matching from.
/// * `mask`  - The mask to use when matching opcodes.
/// * `bytes` - The bytes to match against.
pub unsafe fn compare_mask(addr: *const u8, mask: &'static str, bytes: &'static [u8]) -> bool {
    mask.chars()
        .enumerate()
        // Merge the iterator with the opcodes for matching
        .zip(bytes.iter().copied())
        // Compare the mask and memory at the address with the op codes
        .all(|((offset, mask), op)| mask == '?' || unsafe { *addr.add(offset) } == op)
}

/// MopConfig configures the behavior of the mop function.
pub struct MopConfig<P> {
    /// pointer is the memory address to operate on.
    pub pointer: *const P,

    /// size is the size of the chunk in bytes.
    pub size: usize,

    /// allign_to is an optional boundary to align the chunk's
    /// end address to.
    pub align_to: Option<usize>,

    /// prot_before is the memory protection setting to apply
    /// to the memory chunk before calling op_func.
    pub prot_before: MaybeProt,

    /// prot_after is the memory protection setting to apply
    /// to the memory chunk after op_func returns.
    pub prot_after: MaybeProt,
}

/// MaybeProt specifies the memory protection behavior for the mop function.
pub enum MaybeProt {
    /// DoNoChange tells mop to not change the memory protection
    /// settings of the chunk being operated on.
    DoNotChange,

    /// ChangeTo changes the chunk's memory protection settings to
    /// the specified Prot value.
    ChangeTo(Prot),

    /// RestorePrevious tells mop to restore the chunk's original memory
    /// protection settings.
    ///
    /// This value is only valid for use with the MopConfig.prot_after
    /// field.
    RestorePrevious,
}

/// Prot represents a memory protection setting.
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

/// mop handles the common toil involved in operating on a memory chunk,
/// such as setting the chunk's protection settings before and after
/// operating on it, aligning the chunk's boundaries to a certain bit
/// width, and reading from and writing to the chunk.
///
/// The function works by first applying the config.prot_before memory
/// protection setting to the target memory chunk. The op_func closure
/// is then executed. After op_func finishes running, config.prot_after
/// is applied to the memory chunk.
///
/// ## Safety
///
/// This function is unsafe because it interacts with memory that may be
/// owned by other code or memory that is being operated on concurrently
/// by another thread.
///
/// ## Arguments
///
/// * `config` - A struct that specifies the target memory chunk's
///   boundaries and this function's behavior.
/// * `op_func` - The closure to execute once the config has been
///   applied. The closure will receive an object representing the
///   final address of the memory chunk being operated on after the
///   optional alignment has been applied. The closure can return
///   a result with an error to communicate an error condition back
///   to the code that invoked mop.
#[inline]
pub unsafe fn mop<F, P>(config: MopConfig<P>, op_func: F) -> Result<(), Box<dyn Error>>
where
    F: FnOnce(*mut P) -> Result<(), std::io::Error>,
{
    let mut protect_ptr: *mut P = config.pointer.cast_mut();
    let mut chunk_size = config.size;

    if let Some(align_bits) = config.align_to {
        let adjustment = align_to(protect_ptr, align_bits, config.size);

        protect_ptr = adjustment.new_ptr;
        chunk_size = adjustment.size_to_modify;
    }

    let mut orig_prot: Option<Prot> = None;

    match config.prot_before {
        MaybeProt::RestorePrevious => {
            return Err(format!(
                "prot_before cannot be set to MaybeProt::RestorePrevious"
            ))?;
        }
        MaybeProt::ChangeTo(new_prot) => {
            match protect(protect_ptr, chunk_size, new_prot, None) {
                Ok(i) => {
                    if let Some(old) = i.old {
                        orig_prot = Some(Prot::Custom(old));
                    }
                }
                Err(err) => {
                    return Err(format!(
                        "failed to protect memory region 0x{:x?} (orig: 0x{:x?}) size 0x{:x?} (orig: 0x{:x?}) - {}",
                        protect_ptr.addr(),
                        config.pointer.addr(),
                        chunk_size,
                        config.size,
                        err
                    ))?;
                }
            };
        }
        MaybeProt::DoNotChange => {}
    };

    let func_result = op_func(config.pointer.cast_mut());

    let prot_after_result: Result<ProtectResult, Box<dyn Error>> = match config.prot_after {
        MaybeProt::RestorePrevious => {
            if let Some(orig) = orig_prot {
                protect(protect_ptr, chunk_size, orig, None)
            } else {
                Ok(ProtectResult { old: None })
            }
        }
        MaybeProt::ChangeTo(new_prot) => protect(protect_ptr, chunk_size, new_prot, None),
        MaybeProt::DoNotChange => Ok(ProtectResult { old: None }),
    };

    if let Err(err) = func_result {
        return Err(format!("op_func failed - {err}"))?;
    }

    match prot_after_result {
        Ok(_) => Ok(()),
        Err(err) => {
            return Err(format!(
                "failed to restore memory region protection at {:p} (orig: {:p}) size {:#x} (orig: {:#x}) - {}",
                protect_ptr, config.pointer, chunk_size, config.size, err
            ))?;
        }
    }
}

/// protect modifies the protections of a memory chunk for the current
/// process.
///
/// It provides identical functionality to the mprotect(2) system call
/// on Unix-like systems and the Windows VirtualProtect function.
///
/// ## Safety
///
/// This function is unsafe because it interacts with memory that may be
/// owned by other code or memory that is being operated on concurrently
/// by another thread.
///
/// ## Arguments
///
/// * `pointer`     - The memory address to operate on
/// * `size`        - The size of the memory chunk to operate on
/// * `prot`        - The memory protection to apply to the memory chunk
/// * `allign_with` - An optional boundary to align the chunk to
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

/// ProtectResult captures information after a successful call to
/// the protect function.
pub struct ProtectResult {
    /// old is the previous protection settings for the memory that
    /// protect operated on.
    pub old: Option<u32>,
}

/// AllocFlags are the flags to apply when calling the alloc function.
pub enum AllocFlags {
    Default,
    Custom(u64),
}

/// alloc allocates memory for the current process.
///
/// It provides identical functionality to the mmap(2) system call
/// on Unix-like systems and the Windows VirtualAlloc function.
///
/// ## Arguments
///
/// * `addr` - An optional address to allocate memory on top of.
///   If None, then a new chunk is allocated.
/// * `size` - The size of the allocation in bytes.
/// * `prot` - The memory protection settings to apply to the new chunk.
/// * `flags` - The AllocFlags to use.
pub fn alloc<P>(
    addr: Option<*mut P>,
    size: usize,
    prot: Prot,
    flags: AllocFlags,
) -> Result<*mut P, Box<dyn Error>> {
    #[cfg(unix)]
    let result = unix::alloc(addr, size, prot, flags);

    #[cfg(windows)]
    let result = windows::alloc(addr, size, prot, flags);

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
            };
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

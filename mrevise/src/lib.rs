use std::error::Error;

#[cfg(unix)]
pub mod unix;

#[cfg(windows)]
pub mod windows;

/// find_pattern attempts to find a matching pattern anywhere between
/// the start and end offsets.
///
/// This function was originally written by Jacob T. Read (jacobtread)
/// for their PocketRelay project (Copyright (c) 2023 - 2024 Jacobtread).
///
/// # Safety
///
/// This function is unsafe because it interacts with memory that may be
/// owned by other code or memory that is being operated on concurrently
/// by another thread.
///
/// # Arguments
///
/// * `start_offset` - The address to start matching from.
/// * `end_offset`   - The address to stop matching at.
/// * `mask`         - The mask to use when matching data. Refer to the
///   compare_mask function's rustdoc for an explanation of the mask string.
/// * `bytes`        - The bytes to match against.
///
/// # Examples
///
/// Find the address of a local variable with
/// a value of 0xdeadbeef:
///
/// ```no_run
/// fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let example: u64 = 0xdeadbeef;
///
///     let ptr = std::ptr::addr_of!(example) as usize;
///
///     eprintln!("ptr: {ptr:#x}");
///
///     let result = unsafe {
///         mrevise::find_pattern(ptr, ptr + 0xFF, "????xxxx", &[0xde, 0xad, 0xbe, 0xef])
///     };
///
///     match result {
///         Some(found_ptr) => {
///             let value = unsafe { *(found_ptr as *const u64) };
///
///             eprintln!("found target at: {found_ptr:#x?} - value: {value:#x}");
///         }
///         None => eprintln!("failed to find target :("),
///     }
///
///     Ok(())
/// }
/// ```
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
/// This function was originally written by Jacob T. Read (jacobtread)
/// for their PocketRelay project (Copyright (c) 2023 - 2024 Jacobtread).
///
/// # Safety
///
/// This function is unsafe because it interacts with memory that may be
/// owned by other code or memory that is being operated on concurrently
/// by another thread.
///
/// # Arguments
///
/// * `addr`  - The address to start matching from.
/// * `mask`  - The mask to use when matching data. This string must be
///   the same length as the bytes argument. It can consist of a wildcard
///   character ("?") which indicates any byte can match at the given
///   position or a must-match character (this can be any character other
///   than "?", but is typically just "x").
/// * `bytes` - The bytes to match against.
pub unsafe fn compare_mask(addr: *const u8, mask: &'static str, bytes: &'static [u8]) -> bool {
    mask.chars()
        .enumerate()
        // Merge the iterator with the opcodes for matching
        .zip(bytes.iter().copied())
        // Compare the mask and memory at the address with the op codes
        .all(|((offset, mask), op)| mask == '?' || unsafe { *addr.add(offset) } == op)
}

/// MopConfig defines the bounds of a memory chunk to operate on and
/// configures the behavior of the mop function.
pub struct MopConfig<P> {
    /// pointer is the address of the memory chunk to operate on.
    ///
    /// If you are only interested in expressing a memory address
    /// without any assoicated data type (for example, if the
    /// address is a usize type), this value can be expressed as:
    ///
    /// ```
    /// pointer: addr as *const ()
    /// ```
    ///
    /// ... where "addr" is the name of a usize variable containing
    /// the address. The pointed-to address can be retrived using
    /// Rust's "addr" method.
    pub pointer: *const P,

    /// size is the size of the chunk in bytes.
    pub size: usize,

    /// allign_to is an optional boundary to align the chunk's
    /// end address to.
    ///
    /// This is typically be set to the platform's page size,
    /// which is commonly (but not always!) 4096 bits. Or, in
    /// other words: `Some(4096)` or `Some(0x1000)`
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

/// mop (memory operation) handles the common toil involved in operating
/// on a memory chunk, such as setting the chunk's protection settings
/// before and after operating on it, aligning the chunk's boundaries to
/// a certain bit width, and reading from and writing to the chunk.
///
/// The function works by first applying the config.prot_before memory
/// protection setting to the target memory chunk. The op_func closure
/// is then executed. After op_func finishes running, config.prot_after
/// is applied to the memory chunk.
///
/// # Safety
///
/// This function is unsafe because it interacts with memory that may be
/// owned by other code or memory that is being operated on concurrently
/// by another thread.
///
/// # Arguments
///
/// * `config` - A struct that specifies the target memory chunk's
///   boundaries and this function's behavior.
/// * `op_func` - The closure to execute once the config has been
///   applied. The closure will receive an object representing the
///   final address of the memory chunk being operated on after the
///   optional alignment has been applied. The closure can return
///   a result with an error to communicate an error condition back
///   to the code that invoked mop.
///
/// # Examples
///
/// Change a read-only global variable's value to 0xdeadbeef:
///
/// ```no_run
/// static EXAMPLE: u64 = 0x00;
///
/// fn main() -> Result<(), Box<dyn std::error::Error>> {
///     // This will output: 0x0.
///     eprintln!("value before: {EXAMPLE:#x?}");
///
///     unsafe {
///         mrevise::mop(
///             mrevise::MopConfig {
///                 pointer: std::ptr::addr_of!(EXAMPLE),
///                 size: 8,
///                 align_to: Some(4096),
///                 prot_before: mrevise::MaybeProt::ChangeTo(mrevise::Prot::ReadWrite),
///                 prot_after: mrevise::MaybeProt::ChangeTo(mrevise::Prot::Read),
///             },
///             |example_ptr| {
///                 *example_ptr = 0xdeadbeef;
///
///                 Ok(())
///             },
///         )
///     }?;
///
///     // This outputs: 0xdeadbeef.
///     eprintln!("value after: {EXAMPLE:#x?}");
///
///     Ok(())
/// }
/// ```
#[inline]
pub unsafe fn mop<F, P>(config: MopConfig<P>, op_func: F) -> Result<(), Box<dyn Error>>
where
    F: FnOnce(*mut P) -> Result<(), std::io::Error>,
{
    let mut protect_ptr: *mut P = config.pointer.cast_mut();
    let mut chunk_size = config.size;

    if let Some(align_bits) = config.align_to {
        let adjustment = align_chunk(protect_ptr, align_bits, config.size);

        protect_ptr = adjustment.new_ptr;
        chunk_size = adjustment.new_size;
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

/// protect modifies the protection settings of a memory chunk for the
/// current process.
///
/// It provides identical functionality to the mprotect(2) system call
/// on Unix-like systems and the Windows VirtualProtect function.
///
/// # Safety
///
/// This function is unsafe because it interacts with memory that may be
/// owned by other code or memory that is being operated on concurrently
/// by another thread.
///
/// # Arguments
///
/// * `pointer` - The memory address to operate on.
/// * `size` - The size of the memory chunk to operate on.
/// * `prot` - The memory protection to apply to the memory chunk.
/// * `allign_with` - An optional boundary to align the chunk to. This is
///   typically be set to the platform's page size, which is commonly (but
///   not always!) 4096 bits. Or, in other words: `Some(4096)`
///
/// # Examples
///
/// Change a read-only global variable's value to 0xdeadbeef:
///
/// ```no_run
/// static EXAMPLE: u64 = 0x00;
///
/// fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let ptr = std::ptr::addr_of!(EXAMPLE) as *mut u64;
///
///     // This will output: 0x0.
///     eprintln!("value before: {EXAMPLE:#x?}");
///
///     // If we do not change the memory protection
///     // of the EXAMPLE global variable, this program
///     // will segfault when updating it.
///     mrevise::protect(
///         ptr,
///         std::mem::size_of::<u64>(),
///         mrevise::Prot::ReadWrite,
///         Some(4096),
///     )?;
///
///     unsafe { *ptr = 0xdeadbeef };
///
///     // This will output: 0xdeadbeef.
///     eprintln!("value after: {EXAMPLE:#x?}");
///
///     Ok(())
/// }
/// ```
pub fn protect<P>(
    pointer: *mut P,
    size: usize,
    prot: Prot,
    align_with: Option<usize>,
) -> Result<ProtectResult, Box<dyn Error>> {
    let mut target_ptr = pointer;
    let mut chunk_size = size;

    if let Some(align_bits) = align_with {
        let adjustment = align_chunk(target_ptr.cast(), align_bits, chunk_size);

        target_ptr = adjustment.new_ptr;
        chunk_size = adjustment.new_size;
    }

    #[cfg(unix)]
    let result = unix::protect(target_ptr, chunk_size, prot);

    #[cfg(windows)]
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
/// # Arguments
///
/// * `addr` - An optional address to allocate memory on top of.
///   If None, then a new chunk is allocated.
/// * `size` - The size of the allocation in bytes.
/// * `prot` - The memory protection settings to apply to the new chunk.
/// * `flags` - The AllocFlags to use.
///
/// # Examples
///
/// Allocate a new chunk of memory and tell Rust to treat it as
/// an array of u8 with four elements:
///
/// ```no_run
/// fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let mut chunk = unsafe {
///         *mrevise::alloc::<[u8; 4]>(
///             None,
///             4,
///             mrevise::Prot::ReadWrite,
///             mrevise::AllocFlags::Default,
///         )?
///     };
///
///     chunk[0] = 0xde;
///     chunk[1] = 0xad;
///     chunk[2] = 0xbe;
///     chunk[3] = 0xef;
///
///     // This outputs: [0xde, 0xad, 0xbe, 0xef].
///     eprintln!("{chunk:#x?}");
///
///     Ok(())
/// }
/// ```
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

/// align_chunk aligns a pointer to an arbitrary chunk of memory according
/// to the specified bits. It returns a new pointer and the size of the
/// the new chunk (that is: the original chunk's size plus the length
/// between the aligned pointer and the original pointer).
///
/// # Arguments
///
/// * `pointer` - A pointer to a chunk of memory.
/// * `bits` - The bits to align the pointer to (usually the system's
///   page size - e.g., 4096 or 0x1000).
/// * `chunk_size` - The size of the chunk that pointer is pointing at.
///
/// # Examples
///
/// Align an 8-byte chunk to a boundary of 4096 bits:
///
/// ```no_run
/// fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let mut example: u64 = 0x8badf00d;
///
///     let aligned = mrevise::align_chunk(std::ptr::addr_of_mut!(example), 4096, 8);
///
///     // This outputs:
///     // old ptr: 0x00002bd810fe49e8 | new ptr: 0x00002bd810fe4000 | new chunk size: 2544
///     eprintln!(
///         "old ptr: {:#x?} | new ptr: {:#x?} | new chunk size: {}",
///         std::ptr::addr_of!(example),
///         aligned.new_ptr,
///         aligned.new_size
///     );
///
///     Ok(())
/// }
/// ```
pub fn align_chunk<P>(pointer: *mut P, bits: usize, chunk_size: usize) -> AlignedChunk<P> {
    let current_addr = pointer.addr();

    let new_addr = current_addr & !(bits - 1);

    let diff: usize;

    match new_addr {
        new_addr if current_addr == new_addr => {
            return AlignedChunk {
                new_ptr: pointer,
                new_size: chunk_size,
            };
        }
        new_addr if current_addr > new_addr => diff = current_addr - new_addr,
        _ => diff = new_addr - current_addr,
    };

    AlignedChunk {
        new_ptr: new_addr as *mut P,
        new_size: diff + chunk_size,
    }
}

/// AlignedChunk represents a memory chunk that has been aligned to
/// a bit width and the chunk's new size. The new size is the sum of
/// the old chunk's size plus the length from the aligned chunk pointer
/// to the old pointer.
pub struct AlignedChunk<P> {
    /// new_ptr is the pointer to the chunk after it
    /// has been aligned.
    pub new_ptr: *mut P,

    /// new_size is the size of the chunk after its pointer
    /// has been aligned.
    pub new_size: usize,
}

fn last_error(prefix: &str) -> std::io::Error {
    std::io::Error::new(
        std::io::ErrorKind::Other,
        format!("{prefix} - {err}", err = std::io::Error::last_os_error()),
    )
}

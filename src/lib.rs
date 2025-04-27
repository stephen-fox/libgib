use core::ffi::{c_char, c_int, c_void};

use std::{
    error::Error,
    process::{exit, id},
};

use ctor::ctor;

use mrevise::{use_memory, Prot};

use rsbmalloc::page_allocator;

#[global_allocator]
static ALLOCATOR: page_allocator::PageAllocator = page_allocator::PageAllocator {};

static mut FGETS_PTR: Option<fn(*mut c_void, isize, *mut c_void) -> *mut c_char> = None;
static mut MALLOC_PTR: Option<fn(usize) -> *mut c_void> = None;
static mut SENDFILE_PTR: Option<
    fn(out_fd: c_int, in_fd: c_int, off: *mut i64, count: usize) -> isize,
> = None;

#[ctor]
fn on_load() {
    match library_ctf_on_load_with_err() {
        Ok(_) => {}
        Err(err) => {
            eprintln!("fatal: {err}");
            exit(1);
        }
    }
}

fn library_ctf_on_load_with_err() -> Result<(), Box<dyn Error>> {
    // Based on work by phip1611:
    // https://stackoverflow.com/a/57083797

    eprintln!("DEBUG({}): loading...", id());

    let objects = match unsafe { mmor::objects() } {
        Ok(objs) => objs,
        Err(err) => return Err(format!("failed to get memory-mapped objects - {err}"))?,
    };

    let exe_addr = match objects
        .iter()
        .find(|obj| obj.name.as_ref().is_some_and(|name| name.is_empty()))
    {
        Some(o) => o.addr,
        None => return Err("failed to find exe in memory-mapped objects".into()),
    };

    for object in objects {
        eprintln!("DEBUG: object: 0x{:?} -> 0x{:x?}", object.name, object.addr)
    }

    let got_offset = 0x3f60;

    let got_addr = exe_addr + got_offset;

    let got_size = 0x4000 - got_offset;

    eprintln!("DEBUG: exe: 0x{:x?} | got: 0x{:x?}", exe_addr, got_addr);

    match unsafe {
        use_memory(
            got_addr as *mut c_void,
            mrevise::MemAttrs {
                length: 0x4000 - 0x3f60,
                align_to: Some(4096),
                prot_during: Some(Prot::ReadWrite),
                prot_after: Some(Prot::Read),
                try_restore_orig_prot: false,
            },
            |addr| {
                let mut current = addr.addr();
                let max = addr.addr() + got_size;

                eprintln!("current: 0x{:x?} | max: 0x{:x?}", current, max);

                while current < max {
                    let current_copy = current;

                    let entry_value = read_ptr(current as *mut c_void);

                    current += 8;

                    if entry_value == 0x00 {
                        continue;
                    }

                    let info = match dlrkit::sym_by_addr(entry_value) {
                        Ok(i) => i,
                        Err(err) => {
                            eprintln!("failed to dladdr malloc entry - {err}");
                            continue;
                        }
                    };

                    eprintln!(
                        "DEBUG: addr: 0x{:x?} | name: {} | ptr: 0x{:x?}",
                        current_copy, info.sym_name, info.sym_addr
                    );

                    match info.sym_name.as_str() {
                        "sendfile64" => {
                            let sendfile = swap_got_entry(
                                "sendfile",
                                current_copy as *mut c_void,
                                fake_sendfile as *mut c_void,
                            );
                            SENDFILE_PTR = Some(std::mem::transmute_copy(&sendfile));
                        }
                        _ => {}
                    };
                }
            },
        )
    } {
        Ok(_) => {}
        Err(err) => return Err(format!("failed to modify got - {err}"))?,
    };

    // use std::io::BufRead;
    // let stdin = std::io::stdin();
    // let mut iterator = stdin.lock().lines();
    // iterator.next().unwrap().unwrap();

    eprintln!("DEBUG: load done");

    Ok(())
}

fn plaid_ctf_on_load_with_err() -> Result<(), Box<dyn Error>> {
    // Based on work by phip1611:
    // https://stackoverflow.com/a/57083797

    eprintln!("DEBUG({}): loading...", id());

    let objects = match unsafe { mmor::objects() } {
        Ok(objs) => objs,
        Err(err) => return Err(format!("failed to get memory-mapped objects - {err}"))?,
    };

    let exe_addr = match objects
        .iter()
        .find(|obj| obj.name.as_ref().is_some_and(|name| name.is_empty()))
    {
        Some(o) => o.addr,
        None => return Err("failed to find exe in memory-mapped objects".into()),
    };

    for object in objects {
        eprintln!("DEBUG: object: 0x{:?} -> 0x{:x?}", object.name, object.addr)
    }

    let got_addr = exe_addr + 0x3f58;

    let got = got_addr as *mut c_void;

    eprintln!("DEBUG: exe: 0x{:x?} | got: 0x{:x?}", exe_addr, got_addr);

    // 0x3f58 - 0x4000  .got
    match unsafe {
        use_memory(
            got,
            mrevise::MemAttrs {
                length: 0x4000 - 0x3f58,
                align_to: Some(4096),
                prot_during: Some(Prot::ReadWrite),
                prot_after: Some(Prot::Read),
                try_restore_orig_prot: false,
            },
            |addr| {
                // malloc = got + 0x50.
                let malloc = swap_got_entry("malloc", addr.add(0x50), fake_malloc as *mut c_void);

                MALLOC_PTR = Some(std::mem::transmute_copy(&malloc));

                // fgets = got + 0x40.
                let fgets = swap_got_entry("fgets", addr.add(0x40), fake_fgets as *mut c_void);

                FGETS_PTR = Some(std::mem::transmute_copy(&fgets));
            },
        )
    } {
        Ok(_) => {}
        Err(err) => return Err(format!("failed to modify got - {err}"))?,
    };

    // use std::io::BufRead;
    // let stdin = std::io::stdin();
    // let mut iterator = stdin.lock().lines();
    // iterator.next().unwrap().unwrap();

    eprintln!("DEBUG: load done");

    Ok(())
}

unsafe fn swap_got_entry(name: &str, got_entry: *mut c_void, with: *mut c_void) -> *mut c_void {
    let c_fn_addr = read_ptr(got_entry);

    let fn_ptr = c_fn_addr as *mut c_void;

    let our_fake_fn = with;

    eprintln!(
        "DEBUG: rewrite {} entry: 0x{:x?} -> 0x{:x?}",
        name, c_fn_addr, our_fake_fn
    );

    write_ptr(our_fake_fn, got_entry);

    fn_ptr
}

unsafe fn read_ptr(from: *mut c_void) -> usize {
    let mut addr_bytes: [u8; 8] = [0; 8];

    unsafe {
        std::ptr::copy_nonoverlapping(from as *mut u8, addr_bytes.as_mut_ptr(), addr_bytes.len())
    };

    u64::from_le_bytes(addr_bytes) as usize
}

unsafe fn write_ptr(pointer: *mut c_void, to: *mut c_void) {
    let pointer_bytes = pointer.addr().to_le_bytes();

    unsafe {
        std::ptr::copy_nonoverlapping(pointer_bytes.as_ptr(), to as *mut u8, pointer_bytes.len())
    };
}

extern "C" fn fake_fgets(s: *mut c_void, size: isize, stream: *mut c_void) -> *mut c_char {
    eprintln!(
        "DEBUG: enter fgets(0x{:x?}, {}, 0x{:?})...",
        s.addr(),
        size,
        stream.addr(),
    );

    let f = unsafe { FGETS_PTR.unwrap() };

    let result = f(s, size, stream);

    eprintln!("DEBUG: exit fgets() -> 0x{:x?}", result.addr());

    result
}

extern "C" fn fake_malloc(size: usize) -> *mut c_void {
    eprintln!("DEBUG: enter malloc(0x{:x?})", size);

    let f = unsafe { MALLOC_PTR.unwrap() };

    let result = f(size);

    eprintln!("DEBUG: exit malloc() -> 0x{:x?}", result.addr());

    result
}

// ssize_t isize
// size_t usize
// off64_t i64
extern "C" fn fake_sendfile(out_fd: c_int, in_fd: c_int, off: *mut i64, count: usize) -> isize {
    let offset = unsafe { *off };

    eprintln!(
        "DEBUG: enter sendfile(0x{:x?}, 0x{:x?}, 0x{:x?}, 0x{:x?})",
        out_fd, in_fd, offset, count
    );

    let f = unsafe { SENDFILE_PTR.unwrap() };

    let result = f(out_fd, in_fd, off, count);

    if result < 0 {
        eprintln!(
            "DEBUG: sendfile failed: {}",
            std::io::Error::last_os_error()
        )
    }

    eprintln!(
        "DEBUG: enter sendfile(0x{:x?}, 0x{:x?}, 0x{:x?}, 0x{:x?}) -> 0x{:x?}",
        out_fd, in_fd, offset, count, result
    );

    result
}

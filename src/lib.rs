use std::{
    error::Error,
    ffi::{c_char, c_int, c_void, CString},
    process::{exit, id},
    ptr::null_mut,
};

use ctor::ctor;
use mem::mem::{use_memory, Prot};

mod mem;

// TODO: Try building a static library

static mut FGETS_PTR: Option<fn(*mut c_void, isize, *mut c_void) -> *mut c_void> = None;
static mut MALLOC_PTR: Option<fn(usize) -> *mut c_void> = None;

#[ctor]
fn on_load() {
    match on_load_with_err() {
        Ok(_) => {}
        Err(err) => {
            eprintln!("fatal: {err}");
            exit(1);
        }
    }
}

fn on_load_with_err() -> Result<(), Box<dyn Error>> {
    // Based on work by phip1611:
    // https://stackoverflow.com/a/57083797

    eprintln!("DEBUG({}): loading...", id());

    let self_dl = match Dl::open(None) {
        Ok(h) => h,
        Err(err) => return Err(format!("dlopen self failed - {err}"))?,
    };

    let exe_addr = unsafe { read_ptr(self_dl.handle()) };

    let got_addr = exe_addr + 0x3f58;

    let got = got_addr as *mut c_void;

    eprintln!("exe: 0x{:x?} | got: 0x{:x?}", exe_addr, got_addr);

    // 0x3f58 - 0x4000  .got
    match unsafe {
        use_memory(
            got,
            mem::mem::MemAttrs {
                length: 0x4000 - 0x3f58,
                align_to: Some(4096),
                prot_during: Some(Prot::ReadWrite),
                prot_after: Some(Prot::Read),
                try_restore_orig_prot: false,
            },
            |addr| {
                // malloc = got + 0x50.
                let malloc_entry = addr.add(0x50);

                let malloc_addr = read_ptr(malloc_entry);

                let our_malloc = fake_malloc as *mut c_void;

                eprintln!(
                    "rewrite malloc entry: 0x{:x?} -> 0x{:x?}",
                    malloc_addr, our_malloc
                );

                write_ptr(our_malloc, malloc_entry);
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

    let libc_so = match Dl::open(Some("libc.so.6")) {
        Ok(h) => h,
        Err(err) => return Err(format!("dlopen libc failed - {err}"))?,
    };

    match libc_so.sym("fgets") {
        Ok(f) => unsafe { FGETS_PTR = Some(f) },
        Err(err) => return Err(format!("dlsym fgets failed - {err}"))?,
    };

    match libc_so.sym("malloc") {
        Ok(f) => unsafe { MALLOC_PTR = Some(f) },
        Err(err) => return Err(format!("dlsym fgets failed - {err}"))?,
    };

    eprintln!("DEBUG({}): load done", id());

    Ok(())
}

unsafe fn read_ptr(at: *mut c_void) -> usize {
    let mut addr_bytes: [u8; 8] = [0; 8];

    unsafe {
        std::ptr::copy_nonoverlapping(at as *mut u8, addr_bytes.as_mut_ptr(), addr_bytes.len())
    };

    u64::from_le_bytes(addr_bytes) as usize
}

unsafe fn write_ptr(pointer: *mut c_void, to: *mut c_void) {
    let pointer_bytes = pointer.addr().to_le_bytes();

    unsafe {
        std::ptr::copy_nonoverlapping(pointer_bytes.as_ptr(), to as *mut u8, pointer_bytes.len())
    };
}

#[unsafe(no_mangle)]
extern "C" fn fgets(s: *mut c_void, size: isize, stream: *mut c_void) -> *mut c_void {
    let f = unsafe { FGETS_PTR.unwrap() };

    eprintln!("DEBUG({}): fgets: 0x{:x?} | {}", id(), s.addr(), size);

    f(s, size, stream)
}

extern "C" fn fake_malloc(size: usize) -> *mut c_void {
    let f = unsafe { MALLOC_PTR.unwrap() };

    let result = f(size);

    eprintln!(
        "DEBUG({}): malloc: 0x{:x?} ({}) -> 0x{:x?}",
        id(),
        size,
        size,
        result.addr()
    );

    result
}

const RTLD_NOW: c_int = 0x2;

unsafe extern "C" {
    pub fn dlopen(filename: *const c_char, flag: c_int) -> *mut c_void;

    pub fn dlerror() -> *mut c_char;

    pub fn dlsym(handle: *mut c_void, symbol: *const c_char) -> *mut c_void;

    pub fn dlclose(handle: *mut c_void) -> c_int;
}

struct Dl {
    hnd: *mut c_void,
}

impl Dl {
    fn open(filename: Option<&str>) -> Result<Self, Box<dyn Error>> {
        match do_dlopen(filename) {
            Ok(handle) => Ok(Self { hnd: handle }),
            Err(err) => Err(err),
        }
    }

    fn handle(&self) -> *mut c_void {
        self.hnd
    }

    fn sym<T>(&self, symbol: &str) -> Result<T, Box<dyn Error>> {
        let symbol = CString::new(symbol)?;

        let sym_ptr = unsafe { dlsym(self.hnd, symbol.as_ptr()) };

        if sym_ptr.is_null() {
            match last_dlerror() {
                Some(err_msg) => return Err(err_msg)?,
                None => return Err("dlsym failed without any error (dlerror returned null)".into()),
            }
        }

        // Based on work by Chayim Friedman:
        // https://stackoverflow.com/a/71373744
        let sym_transmute = unsafe { std::mem::transmute_copy(&sym_ptr) };

        Ok(sym_transmute)
    }
}

fn do_dlopen(filename: Option<&str>) -> Result<*mut c_void, Box<dyn Error>> {
    let handle = match filename {
        Some(p) => {
            let path = CString::new(p)?;
            unsafe { dlopen(path.as_ptr(), RTLD_NOW) }
        }
        None => unsafe { dlopen(null_mut(), RTLD_NOW) },
    };

    if handle.is_null() {
        match last_dlerror() {
            Some(err_msg) => return Err(err_msg)?,
            None => return Err("dlopen failed without any error (dlerror returned null)".into()),
        }
    }

    Ok(handle)
}

fn last_dlerror() -> Option<String> {
    let err_ptr = unsafe { dlerror() };
    if err_ptr.is_null() {
        return None;
    }

    match unsafe { CString::from_raw(err_ptr).into_string() } {
        Ok(str) => Some(str),
        Err(_) => Some("failed to convert error into string".into()),
    }
}

use core::ffi::{c_int, c_void};

use std::{
    error::Error,
    marker::PhantomData,
    ops::{Deref, DerefMut},
};

// TODO: Windows support, see GetModuleHandleExW:
// https://learn.microsoft.com/en-us/windows/win32/api/libloaderapi/nf-libloaderapi-getmodulehandleexw
#[cfg(unix)]
pub unsafe fn sym_by_addr(addr: usize) -> Result<SymInfo, Box<dyn Error>> {
    unsafe { unix::sym_by_addr(addr) }
}

pub struct SymInfo {
    pub object_name: String,
    pub object_base_addr: *const c_void,
    pub sym_name: String,
    pub sym_addr: *const c_void,
}

impl std::fmt::Display for SymInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "object_name: '{}' | ", self.object_name)?;

        write!(
            f,
            "object_base_addr: 0x{:x?} | ",
            self.object_base_addr as usize
        )?;

        write!(f, "sym_name: '{}' | ", self.sym_name)?;

        write!(f, "sym_addr: 0x{:x?}", self.sym_addr as usize)?;

        Ok(())
    }
}

pub enum Mode {
    Custom(u32),
}

pub struct Dl {
    hnd: *mut c_void,
}

unsafe impl Send for Dl {}
unsafe impl Sync for Dl {}

impl Dl {
    pub unsafe fn open(file: Option<&str>) -> Result<Self, Box<dyn Error>> {
        #[cfg(unix)]
        unsafe {
            Dl::open_mode(file, unix::RTLD_NOW)
        }

        #[cfg(windows)]
        unsafe {
            Dl::open_mode(file, 0)
        }
    }

    // TODO: Implement custom mode type.
    // TODO: Fix mode arg.
    pub unsafe fn open_mode(file: Option<&str>, mode: c_int) -> Result<Self, Box<dyn Error>> {
        let result;

        #[cfg(unix)]
        unsafe {
            result = unix::do_dlopen(file, mode);
        }

        #[cfg(windows)]
        unsafe {
            result = windows::load_library_exw(file, core::ptr::null_mut(), mode as u32)
        }

        Ok(Self { hnd: result? })
    }

    pub unsafe fn handle(&self) -> *mut c_void {
        self.hnd
    }

    pub unsafe fn sym<T>(&self, symbol_name: &str) -> Result<Symbol<T>, Box<dyn Error>> {
        // This check comes from dlopen2. It ensures that T is
        // the same size as a pointer.
        //
        // Copyright (c) 2017 Szymon Wieloch
        // Copyright (C) 2019 Ahmed Masud <ahmed.masud@saf.ai>
        // Copyright (C) 2022 OpenByte <development.openbyte@gmail.com>
        if size_of::<T>() != size_of::<*mut ()>() {
            panic!("type T has a different size than a pointer");
        }

        let result;

        #[cfg(unix)]
        unsafe {
            result = unix::do_dlsym(self.hnd, symbol_name);
        }

        #[cfg(windows)]
        unsafe {
            result = windows::get_proc_address(self.hnd, symbol_name);
        }

        let sym_ptr = result?;

        // Based on work by Chayim Friedman:
        // https://stackoverflow.com/a/71373744
        let sym_transmute = unsafe { std::mem::transmute_copy(&sym_ptr) };

        Ok(Symbol::new(sym_transmute, sym_ptr as usize))
    }

    pub unsafe fn close(self) -> Result<(), Box<dyn Error>> {
        #[cfg(unix)]
        unsafe {
            unix::do_dlclose(self.hnd)
        }

        #[cfg(windows)]
        unsafe {
            windows::free_library(self.hnd)
        }
    }
}

/// Safe wrapper around a symbol obtained from `Dl`.
///
/// This is the most generic type, valid for obtaining functions,
/// references and pointers. It does not accept null value of
/// the library symbol. Other types may provide more specialized
/// functionality better for some use cases.
///
/// This originally appeared in the dlopen2 Rust library, maintained
/// by OpenByteDev.
///
/// Copyright (c) 2017 Szymon Wieloch
/// Copyright (C) 2019 Ahmed Masud <ahmed.masud@saf.ai>
/// Copyright (C) 2022 OpenByte <development.openbyte@gmail.com>
#[derive(Debug, Clone, Copy)]
pub struct Symbol<'lib, T: 'lib> {
    symbol: T,
    addr: usize,
    pd: PhantomData<&'lib T>,
}

impl<'lib, T> Symbol<'lib, T> {
    pub fn new(symbol: T, addr: usize) -> Symbol<'lib, T> {
        Symbol {
            symbol,
            addr,
            pd: PhantomData,
        }
    }

    pub fn addr(&self) -> usize {
        self.addr
    }
}

impl<'lib, T> Deref for Symbol<'lib, T> {
    type Target = T;
    fn deref(&self) -> &T {
        &self.symbol
    }
}

impl<'lib, T> DerefMut for Symbol<'lib, T> {
    fn deref_mut(&mut self) -> &mut T {
        &mut self.symbol
    }
}

unsafe impl<'lib, T: Send> Send for Symbol<'lib, T> {}
unsafe impl<'lib, T: Sync> Sync for Symbol<'lib, T> {}

#[cfg(unix)]
pub mod unix {
    use core::{
        ffi::{c_char, c_int, c_void, CStr},
        ptr::{null, null_mut},
    };

    use std::{
        error::Error,
        ffi::CString,
        sync::{Mutex, OnceLock},
    };

    use crate::SymInfo;

    pub const RTLD_NOW: c_int = {
        if cfg!(all(target_os = "android", target_pointer_width = "32")) {
            0x00
        } else if cfg!(target_os = "haiku") {
            0x01
        } else {
            0x02
        }
    };

    // This constant's various values comes from
    // the rust-lang/libc library.
    pub const RTLD_DEFAULT: *mut c_void = {
        if cfg!(target_os = "fuchsia") {
            0i64 as *mut c_void
        } else if cfg!(target_os = "aix") {
            -1isize as *mut c_void
        } else if cfg!(any(
            target_os = "freebsd",
            target_os = "netbsd",
            target_os = "openbsd",
            target_os = "ios",
            target_os = "macos",
            target_os = "tvos",
            target_os = "visionos",
        )) {
            -2isize as *mut c_void
        } else if cfg!(target_os = "haiku") {
            0isize as *mut c_void
        } else if cfg!(target_os = "hurd") {
            0i64 as *mut c_void
        } else if cfg!(all(target_os = "android", target_pointer_width = "32")) {
            -1isize as *mut c_void
        } else if cfg!(all(target_os = "android", target_pointer_width = "64")) {
            0i64 as *mut c_void
        } else if cfg!(target_os = "emscripten") {
            0i64 as *mut c_void
        } else if cfg!(target_os = "linux") {
            0i64 as *mut c_void
        } else if cfg!(target_os = "horizon") {
            0 as *mut c_void
        } else if cfg!(target_os = "rtems") {
            -2isize as *mut c_void
        } else if cfg!(target_os = "vita") {
            0 as *mut c_void
        } else if cfg!(target_os = "nto") {
            -2i64 as *mut c_void
        } else if cfg!(target_os = "nuttx") {
            0 as *mut c_void
        } else if cfg!(target_os = "redox") {
            0i64 as *mut c_void
        } else if cfg!(target_os = "solaris") {
            -2isize as *mut c_void
        } else if cfg!(target_os = "vxworks") {
            0i64 as *mut c_void
        } else {
            0x00 as *mut c_void
        }
    };

    extern "C" {
        fn dlopen(file: *const c_char, mode: c_int) -> *mut c_void;

        fn dlerror() -> *mut c_char;

        fn dlsym(handle: *mut c_void, symbol: *const c_char) -> *mut c_void;

        fn dladdr(addr: *const c_void, info: *mut c_void) -> c_int;

        fn dlclose(handle: *mut c_void) -> c_int;
    }

    pub struct DlInfo {
        pub dli_fname: *const c_char,
        pub dli_fbase: *mut c_void,
        pub dli_sname: *const c_char,
        pub dli_saddr: *mut c_void,
    }

    impl DlInfo {
        unsafe fn to_sym_info(&self) -> SymInfo {
            SymInfo {
                object_name: unsafe { const_c_char_to_string(self.dli_fname) },
                object_base_addr: self.dli_fbase,
                sym_name: unsafe { const_c_char_to_string(self.dli_sname) },
                sym_addr: self.dli_saddr,
            }
        }
    }

    unsafe fn const_c_char_to_string(p: *const c_char) -> String {
        if p.is_null() {
            return String::from("");
        }

        unsafe { CStr::from_ptr(p).to_string_lossy().to_string() }
    }

    pub unsafe fn do_dlopen(
        file: Option<&str>,
        mode: c_int,
    ) -> Result<*mut c_void, Box<dyn Error>> {
        let handle = match file {
            Some(p) => {
                let path = CString::new(p)?;

                unsafe { dlopen(path.as_ptr(), mode) }
            }
            None => unsafe { dlopen(null_mut(), mode) },
        };

        if handle.is_null() {
            unsafe {
                match last_dlerror() {
                    Some(err_msg) => return Err(err_msg)?,
                    None => {
                        return Err("dlopen failed without any error (dlerror returned null)".into())
                    }
                }
            }
        }

        Ok(handle)
    }

    pub unsafe fn do_dlsym(
        handle: *mut c_void,
        symbol: &str,
    ) -> Result<*mut c_void, Box<dyn Error>> {
        let name_cstr = CString::new(symbol)?;

        let symbol_ptr = unsafe { dlsym(handle, name_cstr.as_ptr()) };

        if symbol_ptr.is_null() {
            unsafe {
                match last_dlerror() {
                    Some(err_msg) => return Err(err_msg)?,
                    None => {
                        return Err("dlsym failed without any error (dlerror returned null)".into())
                    }
                }
            }
        }

        Ok(symbol_ptr)
    }

    pub(crate) unsafe fn sym_by_addr(addr: usize) -> Result<SymInfo, Box<dyn Error>> {
        let dl_info = unsafe { do_dladdr(addr as *const c_void)? };

        Ok(unsafe { dl_info.to_sym_info() })
    }

    pub unsafe fn do_dladdr(addr: *const c_void) -> Result<DlInfo, Box<dyn Error>> {
        let mut info = DlInfo {
            dli_fname: null(),
            dli_fbase: null_mut(),
            dli_sname: null(),
            dli_saddr: null_mut(),
        };

        let info_ptr: *mut DlInfo = &mut info;

        // We could also use DlInfo in the function siganture,
        // but doing so produces a warning about unsafe FFI.
        // We avoid that warning by using *mut c_void as the
        // datatype and casting to *mut c_void here.
        //
        // Ref Alice's post here:
        // https://users.rust-lang.org/t/extern-block-uses-type-which-is-not-ffi-safe/
        let result = unsafe { dladdr(addr, info_ptr.cast()) };

        if result == 0 {
            unsafe {
                match last_dlerror() {
                    Some(err_msg) => return Err(err_msg)?,
                    None => {
                        return Err("dladdr failed without any error (dlerror returned null)".into())
                    }
                }
            }
        }

        Ok(info)
    }

    pub unsafe fn do_dlclose(handle: *mut c_void) -> Result<(), Box<dyn Error>> {
        let result = unsafe { dlclose(handle) };

        if result != 0 {
            unsafe {
                match last_dlerror() {
                    Some(err_msg) => return Err(err_msg)?,
                    None => {
                        return Err("dlsym failed without any error (dlerror returned null)".into())
                    }
                }
            }
        }

        Ok(())
    }

    static DLERROR_MUTEX: OnceLock<Mutex<u8>> = OnceLock::new();

    unsafe fn last_dlerror() -> Option<String> {
        let mu = DLERROR_MUTEX.get_or_init(|| Mutex::new(0));

        let _mu = mu.lock().unwrap();

        let err_ptr = unsafe { dlerror() };
        if err_ptr.is_null() {
            return None;
        }

        Some(CStr::from_ptr(err_ptr).to_string_lossy().to_string())
    }
}

#[cfg(windows)]
pub mod windows {
    use core::ffi::{c_char, c_void};
    use std::{error::Error, ffi::CString};

    #[link(name = "kernel32")]
    extern "system" {
        fn LoadLibraryExW(
            lp_lib_file_name: *const u16,
            hfile: *mut c_void,
            dwflags: u32,
        ) -> *mut c_void;

        fn GetProcAddress(hmodule: *mut c_void, lp_proc_name: *const c_char) -> *mut c_void;

        fn FreeLibrary(hlibmodule: *mut c_void) -> bool;
    }

    pub unsafe fn load_library_exw(
        lp_lib_file_name: Option<&str>,
        hfile: *mut c_void,
        dwflags: u32,
    ) -> Result<*mut c_void, Box<dyn Error>> {
        if lp_lib_file_name.is_none() {
            return Err("lp_lib_file_name is none")?;
        }

        let lp_lib_file_name = lp_lib_file_name.unwrap();

        let mut lp_lib_file_name_utf16 = lp_lib_file_name.encode_utf16().collect::<Vec<_>>();
        lp_lib_file_name_utf16.push(0);

        let result = LoadLibraryExW(lp_lib_file_name_utf16.as_ptr(), hfile, dwflags);
        if result.is_null() {
            return Err(format!(
                "load library failed - {}",
                std::io::Error::last_os_error()
            ))?;
        }

        Ok(result)
    }

    pub unsafe fn get_proc_address(
        hmodule: *mut c_void,
        lp_proc_name: &str,
    ) -> Result<*mut c_void, Box<dyn Error>> {
        let lp_proc_name = CString::new(lp_proc_name)?;

        let result = GetProcAddress(hmodule, lp_proc_name.as_ptr());
        if result.is_null() {
            return Err(format!(
                "get proc address failed - {}",
                std::io::Error::last_os_error()
            ))?;
        }

        Ok(result)
    }

    pub unsafe fn free_library(hmodule: *mut c_void) -> Result<(), Box<dyn Error>> {
        if !FreeLibrary(hmodule) {
            return Err(format!(
                "failed to free library - {}",
                std::io::Error::last_os_error()
            ))?;
        }

        Ok(())
    }
}

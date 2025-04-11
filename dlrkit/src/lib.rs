use core::ffi::{c_int, c_void};

use std::error::Error;

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

pub struct Dl {
    hnd: *mut c_void,
}

// Full Windows example:
// https://learn.microsoft.com/en-us/windows/win32/dlls/using-run-time-dynamic-linking
impl Dl {
    // TODO: Windows support, see LoadLibraryEx:
    // https://learn.microsoft.com/en-us/windows/win32/api/libloaderapi/nf-libloaderapi-loadlibraryexa
    pub unsafe fn open(file: Option<&str>) -> Result<Self, Box<dyn Error>> {
        unsafe {
            #[cfg(unix)]
            match unix::do_dlopen(file, unix::RTLD_NOW) {
                Ok(handle) => Ok(Self { hnd: handle }),
                Err(err) => Err(err),
            }
        }
    }

    #[cfg(unix)]
    pub unsafe fn open_mode(file: Option<&str>, mode: c_int) -> Result<Self, Box<dyn Error>> {
        unsafe {
            match unix::do_dlopen(file, mode) {
                Ok(handle) => Ok(Self { hnd: handle }),
                Err(err) => Err(err),
            }
        }
    }

    pub unsafe fn handle(&self) -> *mut c_void {
        self.hnd
    }

    // TODO: Windows support, see GetProcAddress:
    // https://learn.microsoft.com/en-us/windows/win32/api/libloaderapi/nf-libloaderapi-getprocaddress
    pub unsafe fn sym<T>(&self, symbol: &str) -> Result<T, Box<dyn Error>> {
        unsafe {
            #[cfg(unix)]
            unix::do_dlsym_transmute::<T>(self.hnd, symbol)
        }
    }

    // TODO: Windows support, see FreeLibrary:
    // https://learn.microsoft.com/en-us/windows/win32/api/libloaderapi/nf-libloaderapi-freelibrary
    pub unsafe fn close(self) -> Result<(), Box<dyn Error>> {
        unsafe {
            #[cfg(unix)]
            unix::do_dlclose(self.hnd)
        }
    }
}

#[cfg(unix)]
pub mod unix {
    use core::{
        ffi::{c_char, c_int, c_void},
        ptr::{null, null_mut},
    };

    use std::{error::Error, ffi::CString};

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

    #[cfg(target_os = "fuchsia")]
    pub const RTLD_DEFAULT: *mut c_void = 0i64 as *mut c_void;
    #[cfg(target_os = "aix")]
    pub const RTLD_DEFAULT: *mut c_void = -1isize as *mut c_void;
    #[cfg(any(target_os = "freebsd", target_os = "netbsd", target_os = "openbsd"))]
    pub const RTLD_DEFAULT: *mut c_void = -2isize as *mut c_void;
    #[cfg(target_os = "cygwin")]
    pub const RTLD_DEFAULT: *mut c_void = 0isize as *mut c_void;
    #[cfg(target_os = "haiku")]
    pub const RTLD_DEFAULT: *mut c_void = 0isize as *mut c_void;
    #[cfg(target_os = "hurd")]
    pub const RTLD_DEFAULT: *mut c_void = 0i64 as *mut c_void;
    #[cfg(all(target_os = "android", target_pointer_width = "32"))]
    pub const RTLD_DEFAULT: *mut c_void = -1isize as *mut c_void;
    #[cfg(all(target_os = "android", target_pointer_width = "64"))]
    pub const RTLD_DEFAULT: *mut c_void = 0i64 as *mut c_void;
    #[cfg(target_os = "emscripten")]
    pub const RTLD_DEFAULT: *mut c_void = 0i64 as *mut c_void;
    #[cfg(target_os = "linux")]
    pub const RTLD_DEFAULT: *mut c_void = 0i64 as *mut c_void;
    #[cfg(target_os = "horizon")]
    pub const RTLD_DEFAULT: *mut c_void = 0 as *mut c_void;
    #[cfg(target_os = "rtems")]
    pub const RTLD_DEFAULT: *mut c_void = -2isize as *mut c_void;
    #[cfg(target_os = "vita")]
    pub const RTLD_DEFAULT: *mut c_void = 0 as *mut c_void;
    #[cfg(target_os = "nto")]
    pub const RTLD_DEFAULT: *mut c_void = -2i64 as *mut c_void;
    #[cfg(target_os = "nuttx")]
    pub const RTLD_DEFAULT: *mut c_void = 0 as *mut c_void;
    #[cfg(target_os = "redox")]
    pub const RTLD_DEFAULT: *mut c_void = 0i64 as *mut c_void;
    #[cfg(target_os = "solaris")]
    pub const RTLD_DEFAULT: *mut c_void = -2isize as *mut c_void;
    #[cfg(target_os = "vxworks")]
    pub const RTLD_DEFAULT: *mut c_void = 0i64 as *mut c_void;

    unsafe extern "C" {
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

        let cs = unsafe { CString::from_raw(p.cast_mut()) };

        cs.into_string().unwrap_or_default()
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

    pub unsafe fn do_dlsym_transmute<T>(
        handle: *mut c_void,
        symbol: &str,
    ) -> Result<T, Box<dyn Error>> {
        let sym_ptr = unsafe { do_dlsym(handle, symbol)? };

        // Based on work by Chayim Friedman:
        // https://stackoverflow.com/a/71373744
        let sym_transmute = unsafe { std::mem::transmute_copy(&sym_ptr) };

        Ok(sym_transmute)
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

    unsafe fn last_dlerror() -> Option<String> {
        let err_ptr = unsafe { dlerror() };
        if err_ptr.is_null() {
            return None;
        }

        match unsafe { CString::from_raw(err_ptr).into_string() } {
            Ok(str) => Some(str),
            Err(_) => Some("failed to convert error into string".into()),
        }
    }
}

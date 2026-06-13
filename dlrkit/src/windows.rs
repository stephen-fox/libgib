use core::ffi::{c_char, c_void};

use std::{error::Error, ffi::CString, path::Path};

use crate::OpenMode;

// GetModuleHandleExW constants:
pub const GET_MODULE_HANDLE_EX_FLAG_FROM_ADDRESS: u32 = 0x00000004;
pub const GET_MODULE_HANDLE_EX_FLAG_UNCHANGED_REFCOUNT: u32 = 0x00000002;

#[link(name = "kernel32")]
extern "system" {
    fn GetModuleHandleExW(
        dwflags: u32,
        lp_module_name: *const u16,
        ph_module: *mut *mut c_void,
    ) -> bool;

    fn LoadLibraryExW(
        lp_lib_file_name: *const u16,
        hfile: *mut c_void,
        dwflags: u32,
    ) -> *mut c_void;

    fn GetProcAddress(hmodule: *mut c_void, lp_proc_name: *const c_char) -> *mut c_void;

    fn FreeLibrary(hlibmodule: *mut c_void) -> bool;
}

pub unsafe fn load_library_or_get_module_handle<P: AsRef<Path>>(
    lp_lib_file_name: Option<P>,
    mode: OpenMode,
) -> Result<*mut c_void, Box<dyn Error>> {
    let maybe_path = path_to_utf16(lp_lib_file_name)
        .map_err(|err| format!("failed to convert lp_lib_file_name to utf-16 - {err}"))?;

    let dwflags: u32 = match mode {
        OpenMode::Win32(v) => v,
        _ => return Err(format!("unknown open mode: {mode}").into()),
    };

    match maybe_path {
        Some(path) => load_library_exw(path, core::ptr::null_mut(), dwflags),
        None => get_module_handle_exw(dwflags, None),
    }
}

pub enum GetModuleHandleModuleName {
    Name(Vec<u16>),
    Address(usize),
}

pub unsafe fn get_module_handle_exw(
    dwflags: u32,
    lp_module_name: Option<GetModuleHandleModuleName>,
) -> Result<*mut c_void, Box<dyn Error>> {
    let lp_module_name_ptr: *const u16 = match lp_module_name {
        Some(path) => match path {
            GetModuleHandleModuleName::Name(name) => name.as_ptr(),
            GetModuleHandleModuleName::Address(addr) => addr as *const u16,
        },
        None => core::ptr::null_mut(),
    };

    let mut ph_module: *mut c_void = core::ptr::null_mut();

    if !GetModuleHandleExW(dwflags, lp_module_name_ptr, &mut ph_module) {
        return Err(format!(
            "get_module_handle_exw failed - {}",
            std::io::Error::last_os_error()
        ))?;
    }

    Ok(ph_module)
}

pub unsafe fn load_library_exw(
    lp_lib_file_name: Vec<u16>,
    hfile: *mut c_void,
    dwflags: u32,
) -> Result<*mut c_void, Box<dyn Error>> {
    let result = LoadLibraryExW(lp_lib_file_name.as_ptr(), hfile, dwflags);
    if result.is_null() {
        return Err(format!(
            "load_library_exw failed - {}",
            std::io::Error::last_os_error()
        ))?;
    }

    Ok(result)
}

pub fn path_to_utf16<P: AsRef<Path>>(path: Option<P>) -> Result<Option<Vec<u16>>, Box<dyn Error>> {
    match path {
        Some(p) => match p.as_ref().to_str() {
            Some(s) => {
                let mut utf16 = s.encode_utf16().collect::<Vec<_>>();
                utf16.push(0);

                Ok(Some(utf16))
            }
            None => return Err("str conversaion failed")?,
        },
        None => Ok(None),
    }
}

pub unsafe fn get_proc_address(
    hmodule: *mut c_void,
    lp_proc_name: &str,
) -> Result<*mut c_void, Box<dyn Error>> {
    let lp_proc_name = CString::new(lp_proc_name)?;

    let result = GetProcAddress(hmodule, lp_proc_name.as_ptr());
    if result.is_null() {
        return Err(format!(
            "get_proc_address failed - {}",
            std::io::Error::last_os_error()
        ))?;
    }

    Ok(result)
}

pub unsafe fn free_library(hmodule: *mut c_void) -> Result<(), Box<dyn Error>> {
    if !FreeLibrary(hmodule) {
        return Err(format!(
            "free_library failed - {}",
            std::io::Error::last_os_error()
        ))?;
    }

    Ok(())
}

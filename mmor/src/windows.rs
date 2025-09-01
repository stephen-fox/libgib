#![allow(non_snake_case)]

use core::{ffi::c_void, ptr};

use std::{error::Error, mem::size_of, path::PathBuf};

use crate::{path_basename, Object};

const LIST_MODULES_ALL: u32 = 0x03;

#[link(name = "kernel32")]
unsafe extern "system" {
    /// Returns a HANDLE.
    fn GetCurrentProcess() -> *mut c_void;

    /// * HANDLE hProcess:
    ///   A handle to the process
    /// * HMODULE lpHmodule:
    ///   An array that receives the list of module handles.
    /// * DWORD cb:
    ///   The size of the lphModule array, in bytes.
    /// * LPDWORD lpcbNeeded:
    ///   The number of bytes required to store all module handles
    ///   in the lphModule array.
    /// * DWORD dwFilterFlag:
    ///   The filter criteria.
    fn K32EnumProcessModulesEx(
        hProcess: *mut c_void,
        lpHmodule: *mut c_void,
        cb: u32,
        lpcbNeeded: *mut u32,
        dwFilterFlag: u32,
    ) -> i32;

    /// * HANDLE  hProcess:
    ///   A handle to the process that contains the module.
    /// * HMODULE hModule:
    ///   A handle to the module.
    /// * LPWSTR  lpFilename:
    ///   A pointer to a buffer that receives the fully qualified path
    ///   to the module.
    /// * DWORD   nSize:
    ///   The size of the lpFilename buffer, in characters.
    fn K32GetModuleFileNameExW(
        hProcess: *mut c_void,
        hModule: *mut c_void,
        lpFilename: *const u16,
        nSize: u32,
    ) -> u32;

    /// * HANDLE  hProcess:
    ///   A handle to the process that contains the module.
    /// * HMODULE hModule:
    ///   A handle to the module.
    /// * LPMODULEINFO lpmodinfo
    ///   A pointer to the MODULEINFO structure that receives
    ///   information about the module.
    /// * DWORD cb:
    ///   The size of the MODULEINFO structure, in bytes.
    fn K32GetModuleInformation(
        hProcess: *mut c_void,
        hModule: *mut c_void,
        lpModuleInfo: *mut MODULEINFO,
        cb: u32,
    ) -> bool;
}

pub unsafe fn objects() -> Result<Vec<Object>, Box<dyn Error>> {
    let current_process = unsafe { GetCurrentProcess() };
    if current_process.is_null() {
        Err(format!(
            "GetCurrentProcess failed - {err}",
            err = std::io::Error::last_os_error()
        ))?
    }

    let total_modules = total_enum_process_modules_ex(current_process)
        .map_err(|err| format!("failed to get total number of loaded modules - {err}"))?;

    // Passing an array (Vec) via FFI by Michael-F-Bryan:
    // https://users.rust-lang.org/t/ffi-how-to-pass-a-array-with-structs-to-a-c-func-that-fills-the-array-out-pointer-and-then-how-to-access-the-items-after-in-my-rust-code/83798/2
    let mut modules: Vec<*mut c_void> = Vec::with_capacity(total_modules);

    let modules_uninit = modules.spare_capacity_mut();

    let mut num_modules_returned: u32 = 0;

    let enum_modules_res = unsafe {
        K32EnumProcessModulesEx(
            current_process,
            modules_uninit.as_mut_ptr().cast(),
            modules_uninit.len() as u32,
            &mut num_modules_returned,
            LIST_MODULES_ALL,
        )
    };
    if enum_modules_res == 0 {
        Err(format!(
            "K32EnumProcessModulesEx failed - {err}",
            err = std::io::Error::last_os_error()
        ))?
    }

    unsafe { modules.set_len(num_modules_returned as usize) };

    let mut objects = Vec::new();

    for module_handle in modules {
        let object = module_to_object(current_process, module_handle)
            .map_err(|err| format!("failed to lookup windows module information - {err}"))?;

        objects.push(object);
    }

    Ok(objects)
}

fn total_enum_process_modules_ex(process_handle: *mut c_void) -> Result<usize, Box<dyn Error>> {
    let mut num_bytes_needed: u32 = 0;

    let enum_modules_res = unsafe {
        K32EnumProcessModulesEx(
            process_handle,
            ptr::null_mut(),
            0,
            &mut num_bytes_needed,
            LIST_MODULES_ALL,
        )
    };
    if enum_modules_res == 0 {
        Err(format!(
            "K32EnumProcessModulesEx failed - {err}",
            err = std::io::Error::last_os_error()
        ))?
    }

    Ok(num_bytes_needed as usize / size_of::<*mut c_void>())
}

fn module_to_object(
    process_handle: *mut c_void,
    module_handle: *mut c_void,
) -> Result<Object, Box<dyn Error>> {
    let info = get_module_info(process_handle, module_handle)
        .map_err(|err| format!("get module information failed - {err}"))?;

    const MAX_PATH: usize = 32767;

    let mut filename_raw: Vec<u16> = Vec::with_capacity(MAX_PATH);

    let filename_raw_uninit = filename_raw.spare_capacity_mut();

    let filename_res = unsafe {
        K32GetModuleFileNameExW(
            process_handle,
            module_handle,
            filename_raw_uninit.as_mut_ptr().cast(),
            filename_raw_uninit.len() as u32,
        )
    };
    if filename_res == 0 {
        Err(format!(
            "K32GetModuleFileNameExW failed - {err}",
            err = std::io::Error::last_os_error()
        ))?
    }

    unsafe { filename_raw.set_len(filename_res as usize) };

    match String::from_utf16(&filename_raw) {
        Ok(s) => {
            let module_path = PathBuf::from(s);

            Ok(Object {
                name: path_basename(&module_path),
                path: Some(module_path),
                addr: info.lpBaseOfDll as usize,
            })
        }
        Err(err) => Err(err)?,
    }
}

#[repr(C)]
#[derive(Clone, Copy)]
struct MODULEINFO {
    pub lpBaseOfDll: *mut c_void,
    pub SizeOfImage: u32,
    pub EntryPoint: *mut c_void,
}

fn get_module_info(
    process_handle: *mut c_void,
    module_handle: *mut c_void,
) -> Result<MODULEINFO, Box<dyn Error>> {
    let mut mod_info = MODULEINFO {
        lpBaseOfDll: ptr::null_mut(),
        SizeOfImage: 0,
        EntryPoint: ptr::null_mut(),
    };

    let result = unsafe {
        K32GetModuleInformation(
            process_handle,
            module_handle,
            &mut mod_info,
            size_of::<MODULEINFO>() as u32,
        )
    };
    if !result {
        Err(format!(
            "K32GetModuleInformation failed - {err}",
            err = std::io::Error::last_os_error()
        ))?
    }

    Ok(mod_info)
}

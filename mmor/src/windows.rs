#![allow(non_snake_case)]

use core::{ffi::c_void, ptr};

use std::{error::Error, mem::size_of, path::PathBuf};

use crate::{path_basename, Object, ObjectLookupOptions, Objects};

const ENUM_PROCESS_MODULES_FILTER_FLAG: u32 = {
    if cfg!(target_pointer_width = "32") {
        0x01 // LIST_MODULES_32BIT
    } else {
        0x03 // LIST_MODULES_ALL
    }
};

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

pub unsafe fn objects(options: ObjectLookupOptions) -> Result<Objects, Box<dyn Error>> {
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
    let mut module_handles: Vec<*mut c_void> = Vec::with_capacity(total_modules);

    let modules_uninit = module_handles.spare_capacity_mut();

    let mut num_bytes_needed: u32 = 0;

    let enum_modules_res = unsafe {
        K32EnumProcessModulesEx(
            current_process,
            modules_uninit.as_mut_ptr().cast(),
            modules_uninit.len() as u32,
            &mut num_bytes_needed,
            ENUM_PROCESS_MODULES_FILTER_FLAG,
        )
    };
    if enum_modules_res == 0 {
        Err(format!(
            "K32EnumProcessModulesEx failed - {err}",
            err = std::io::Error::last_os_error()
        ))?
    }

    unsafe { module_handles.set_len(total_modules as usize) };

    let mut objects = Vec::new();

    for (i, module_handle) in module_handles.iter_mut().enumerate() {
        match lookup_module(current_process, *module_handle) {
            Ok(object) => objects.push(object),
            Err(err) => {
                if options.skip_invalid_handle && err.is_invalid_handle_error() {
                    continue;
                }

                return Err(format!(
                    "failed to lookup windows module information (i: {i}, handle: {module_handle:p}) - {err}",
                ))?;
            }
        }
    }

    Ok(Objects { objects: objects })
}

fn total_enum_process_modules_ex(process_handle: *mut c_void) -> Result<usize, Box<dyn Error>> {
    let mut num_bytes_needed: u32 = 0;

    let enum_modules_res = unsafe {
        K32EnumProcessModulesEx(
            process_handle,
            ptr::null_mut(),
            0,
            &mut num_bytes_needed,
            ENUM_PROCESS_MODULES_FILTER_FLAG,
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

fn lookup_module(
    process_handle: *mut c_void,
    module_handle: *mut c_void,
) -> Result<Object, LookupModuleError> {
    let info = get_module_info(process_handle, module_handle)
        .map_err(|err| LookupModuleError::GetModuleInfoFailed(err))?;

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
        Err(LookupModuleError::GetModuleFileNameFailed(
            std::io::Error::last_os_error(),
        ))?;
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
        Err(err) => Err(LookupModuleError::ParseFileNameFailed(err))?,
    }
}

enum LookupModuleError {
    GetModuleInfoFailed(std::io::Error),
    GetModuleFileNameFailed(std::io::Error),
    ParseFileNameFailed(std::string::FromUtf16Error),
}

impl std::fmt::Display for LookupModuleError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Self::GetModuleInfoFailed(err) => write!(f, "get module info failed - {err}"),
            Self::ParseFileNameFailed(err) => {
                write!(f, "failed to convert filename to utf16 string - {err}")
            }
            Self::GetModuleFileNameFailed(err) => write!(f, "get module file name failed - {err}"),
        }
    }
}

impl LookupModuleError {
    fn is_invalid_handle_error(&self) -> bool {
        let err = match self {
            Self::GetModuleInfoFailed(err) => err,
            Self::GetModuleFileNameFailed(err) => err,
            _ => return false,
        };

        // 6 seems to be invalid handle error.
        err.raw_os_error().is_some_and(|raw_err| raw_err == 6)
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
) -> Result<MODULEINFO, std::io::Error> {
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
        return Err(std::io::Error::last_os_error())?;
    }

    Ok(mod_info)
}

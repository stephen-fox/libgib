use core::ffi::c_void;

use std::{
    error::Error,
    ffi::{CStr, c_char},
    path::PathBuf,
};

use crate::{Object, ObjectLookupOptions, Objects, SymInfo, path_basename};

pub unsafe fn objects(_: ObjectLookupOptions) -> Result<Objects, Box<dyn Error>> {
    let mut objects = Objects {
        objects: Vec::new(),
    };

    let objects_ptr: *mut Objects = &mut objects;

    unsafe { libc::dl_iterate_phdr(Some(callback), objects_ptr as *mut c_void) };

    Ok(objects)
}

unsafe extern "C" fn callback(
    info_ptr: *mut libc::dl_phdr_info,
    _: usize,
    data_ptr: *mut c_void,
) -> i32 {
    if info_ptr.is_null() {
        return 0;
    }

    if data_ptr.is_null() {
        return 0;
    }

    let objects_ptr = data_ptr as *mut Objects;

    let objs = unsafe { &mut *objects_ptr };

    let info = unsafe { *info_ptr };

    let mut name: Option<String> = None;

    let mut path: Option<PathBuf> = None;

    if !info.dlpi_name.is_null() {
        let tmp = unsafe { CStr::from_ptr(info.dlpi_name) };

        if let Ok(str_ref) = tmp.to_str() {
            if str_ref.starts_with("/") {
                let path_buf = PathBuf::from(str_ref);

                name = path_basename(&path_buf);

                path = Some(path_buf);
            } else {
                name = Some(str_ref.to_string());
            }
        }
    }

    objs.objects.push(Object {
        name: name,
        path: path,
        addr: info.dlpi_addr as usize,
    });

    0
}

pub unsafe fn sym_from_addr(addr: usize) -> Result<SymInfo, Box<dyn Error>> {
    let dl_info = unsafe { dlrkit::unix::do_dladdr(addr as *const c_void)? };

    Ok(unsafe { dlinfo_to_sym_info(dl_info) })
}

unsafe fn dlinfo_to_sym_info(info: dlrkit::unix::DlInfo) -> SymInfo {
    SymInfo {
        object_name: unsafe { const_c_char_to_string(info.dli_fname) },
        object_base_addr: info.dli_fbase.addr(),
        sym_name: unsafe { const_c_char_to_string(info.dli_sname) },
        sym_addr: info.dli_saddr.addr(),
    }
}

unsafe fn const_c_char_to_string(ptr: *const c_char) -> String {
    if ptr.is_null() {
        return String::from("");
    }

    unsafe { CStr::from_ptr(ptr).to_string_lossy().to_string() }
}

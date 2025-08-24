use core::ffi::c_void;

use std::{error::Error, ffi::CStr};

use crate::{Object, Objects};

pub unsafe fn objects() -> Result<Vec<Object>, Box<dyn Error>> {
    let mut objs = Objects {
        objects: Vec::new(),
    };

    let objects_ptr: *mut Objects = &mut objs;

    unsafe { libc::dl_iterate_phdr(Some(callback), objects_ptr as *mut c_void) };

    Ok(objs.objects)
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

    if !info.dlpi_name.is_null() {
        let tmp = unsafe { CStr::from_ptr(info.dlpi_name) };

        if let Ok(str) = tmp.to_str() {
            name = Some(str.to_string());
        }
    }

    objs.objects.push(Object {
        name: name,
        addr: info.dlpi_addr as usize,
    });

    0
}

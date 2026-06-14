#![allow(non_snake_case)]

use core::{error::Error, ffi::c_void};
use server::Server;
use std::sync::OnceLock;

mod proxyfunctions;

// sym.imp.VCRUNTIME140.dll_memcpy - base_addr = offset
// 0x009f2458                      - 0x400000  = 0x5F2458
const MEMCPY_IMPORT_PTR_OFFSET: usize = 0x5F2458;

static SERVER: OnceLock<Server> = OnceLock::new();

static MEMCPY_PTR: OnceLock<MemcpySig> = OnceLock::new();
type MemcpySig = extern "C" fn(dst: *mut c_void, src: *mut c_void, nbytes: usize) -> *mut c_void;

#[link(name = "user32")]
unsafe extern "system" {
    fn MessageBoxW(hwnd: isize, lptext: *const u16, lpcaption: *const u16, utype: u32) -> i32;
}

#[unsafe(no_mangle)]
unsafe extern "system" fn DllMain(_: isize, call_reason: u32, _: *mut ()) -> bool {
    // https://learn.microsoft.com/en-us/windows/win32/dlls/dllmain
    const DLL_PROCESS_ATTACH: u32 = 1;

    if call_reason == DLL_PROCESS_ATTACH {
        attach();
    }

    true
}

fn attach() {
    #[cfg(feature = "debug")]
    dbg_msg_box(format!(
        "loaded into: '{}'",
        std::env::args().collect::<Vec<_>>().join(" ")
    ));

    if let Err(err) = attach_with_error() {
        err_msg_box(format!("error: {err}"));
    }
}

fn attach_with_error() -> Result<(), Box<dyn Error>> {
    let objects = unsafe {
        mmor::objects_with_options(mmor::ObjectLookupOptions {
            skip_invalid_handles: true,
        })
        .map_err(|err| format!("failed to get memory-mapped objects - {err}"))?
    };

    let aomx_obj = match objects.objects.iter().find(|obj| {
        obj.name
            .as_ref()
            .is_some_and(|name| name.eq_ignore_ascii_case("aomx.exe"))
    }) {
        Some(obj) => obj,
        None => {
            return Err("failed to find aomx.exe memory-mapped object")?;
        }
    };

    let srv = server::start().map_err(|err| format!("failed to start server - {err}"))?;
    let _ = SERVER.set(srv);

    let memcpy_import_ptr: *const u32 =
        core::ptr::with_exposed_provenance_mut(aomx_obj.addr + MEMCPY_IMPORT_PTR_OFFSET);

    let memcpy_addr = unsafe { memcpy_import_ptr.read() };

    let symbolizer = unsafe {
        mmor::Symbolizer::new().map_err(|err| format!("failed to create symbolizer - {err}"))?
    };

    let mc_info = unsafe {
        symbolizer
            .by_addr(memcpy_addr as usize)
            .map_err(|err| format!("failed to lookup memcpy - {err}"))?
    };

    dbg_msg_box(format!("info: {}", mc_info));

    unsafe { MEMCPY_PTR.get_or_init(|| std::mem::transmute_copy(&memcpy_addr)) };

    unsafe {
        mrevise::mop(
            mrevise::MopConfig {
                pointer: memcpy_import_ptr,
                size: 4,
                align_to: None,
                prot_before: mrevise::MaybeProt::ChangeTo(mrevise::Prot::ReadWrite),
                prot_after: mrevise::MaybeProt::DoNotChange,
            },
            |addr| {
                let fake = fake_memcpy as *const ();
                *addr = fake.addr() as u32;
                Ok(())
            },
        )
        .map_err(|err| format!("failed to write fake memcpy address - {err}"))?;
    }

    dbg_msg_box(format!(
        "memcpy: import: {memcpy_import_ptr:p} | addr: {memcpy_addr:#x}"
    ));

    Ok(())
}

extern "C" fn fake_memcpy(dst: *mut c_void, src: *mut c_void, nbytes: usize) -> *mut c_void {
    let memcpy = MEMCPY_PTR.get().unwrap();
    let srv = SERVER.get().unwrap();
    srv.handle_fake_memcpy(dst, src, nbytes);
    memcpy(dst, src, nbytes)
}

fn err_msg_box(msg: String) {
    msg_box(format!("🤕 {msg}"));
}

fn dbg_msg_box(msg: String) {
    msg_box(format!("debug: {msg}"))
}

fn msg_box(msg: String) {
    // https://github.com/microsoft/windows-rs/issues/973#issuecomment-1363481060
    let mut msg = msg.encode_utf16().collect::<Vec<_>>();
    msg.push(0x00);

    let mut title = format!("{} {}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"))
        .encode_utf16()
        .collect::<Vec<_>>();

    title.push(0x00);

    unsafe {
        MessageBoxW(0, msg.as_ptr(), title.as_ptr(), 0);
    };
}

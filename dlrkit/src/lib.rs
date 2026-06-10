use core::ffi::{c_int, c_void};

use std::{
    error::Error,
    marker::PhantomData,
    ops::{Deref, DerefMut},
    path::Path,
};

#[cfg(unix)]
pub mod unix;

#[cfg(windows)]
pub mod windows;

/// sym_by_addr looks up the symbol corresponding to the specified
/// memory address.
///
/// ## Safety
///
/// This function is unsafe because it relies on OS APIs that
/// provide no memory safety assurances.
///
/// ## Arguments
///
/// * `addr` - The memory address of the symbol to lookup.
///
/// TODO: Windows support, see GetModuleHandleExW:
/// https://learn.microsoft.com/en-us/windows/win32/api/libloaderapi/nf-libloaderapi-getmodulehandleexw
#[cfg(unix)]
pub unsafe fn sym_by_addr(addr: usize) -> Result<SymInfo, Box<dyn Error>> {
    unsafe { unix::sym_by_addr(addr) }
}

/// SymInfo represents information about a symbol.
pub struct SymInfo {
    /// object_name is the name of the symbol's parent object.
    pub object_name: String,

    /// object_base_addr is the base address of the symbol's
    /// parent object.
    pub object_base_addr: *const c_void,

    /// sym_name is the name of the symbol.
    pub sym_name: String,

    /// sym_addr is the address of the symbol.
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

/// OpenMode specifies the behavior for the Dl::open_mode function.
pub enum OpenMode {
    Unix(c_int),
    Win32(u32),
}

impl std::fmt::Display for OpenMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OpenMode::Unix(_) => write!(f, "unix")?,
            OpenMode::Win32(_) => write!(f, "windows")?,
        }

        Ok(())
    }
}

/// Dl represents a dynamic linker object such as a library.
pub struct Dl {
    hnd: *mut c_void,
}

unsafe impl Send for Dl {}
unsafe impl Sync for Dl {}

impl Dl {
    /// open loads a library such as a .so file on Unix-like systems or
    /// a .dll on Windows. It is a wrapper for the open_mode function,
    /// refer to that function's documentation for more details.
    ///
    /// ## Safety
    ///
    /// This function is unsafe because it relies on OS APIs that
    /// provide no memory safety assurances.
    pub unsafe fn open<P: AsRef<Path>>(file: Option<P>) -> Result<Self, Box<dyn Error>> {
        #[cfg(unix)]
        unsafe {
            Dl::open_mode(file, OpenMode::Unix(unix::RTLD_NOW))
        }

        #[cfg(windows)]
        unsafe {
            Dl::open_mode(file, OpenMode::Win32(0))
        }
    }

    /// open loads a library such as a .so file on Unix-like systems or
    /// a .dll on Windows.
    ///
    /// ## Safety
    ///
    /// This function is unsafe because it relies on OS APIs that
    /// provide no memory safety assurances.
    ///
    /// ## Arguments
    ///
    /// * file - The path to the dynamic library to load. On Unix-like
    ///   systems this can be set to None, in which case a pointer to
    ///   the process' executable will be returned.
    /// * mode - Configures the behavior of the dynamic linker when
    ///   loading the library.
    pub unsafe fn open_mode<P: AsRef<Path>>(
        file: Option<P>,
        mode: OpenMode,
    ) -> Result<Self, Box<dyn Error>> {
        let result;

        #[cfg(unix)]
        unsafe {
            result = unix::do_dlopen(file, mode);
        }

        #[cfg(windows)]
        unsafe {
            result = windows::load_library_exw(file, core::ptr::null_mut(), mode)
        }

        Ok(Self { hnd: result? })
    }

    /// handle returns the underlying pointer to the memory-mapped object.
    pub unsafe fn handle(&self) -> *mut c_void {
        self.hnd
    }

    /// sym looks for a symbol in the current object by its name.
    ///
    /// ## Safety
    ///
    /// This function is unsafe because it relies on OS APIs that
    /// provide no memory safety assurances.
    ///
    /// ## Arguments
    ///
    /// * `symbol_name` - The name of the symbol to lookup.
    pub unsafe fn sym<T>(&self, symbol_name: &str) -> Result<Sym<'_, T>, Box<dyn Error>> {
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

        Ok(Sym::new(sym_transmute, sym_ptr as usize))
    }

    /// close unloads the underlying memory-mapped object.
    ///
    /// ## Safety
    ///
    /// This function is unsafe because it relies on OS APIs that
    /// provide no memory safety assurances.
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

/// Sym is a safe wrapper around a symbol obtained from `Dl`.
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
pub struct Sym<'lib, T: 'lib> {
    symbol: T,
    addr: usize,
    pd: PhantomData<&'lib T>,
}

impl<'lib, T> Sym<'lib, T> {
    pub fn new(symbol: T, addr: usize) -> Sym<'lib, T> {
        Sym {
            symbol,
            addr,
            pd: PhantomData,
        }
    }

    pub fn addr(&self) -> usize {
        self.addr
    }
}

impl<'lib, T> Deref for Sym<'lib, T> {
    type Target = T;
    fn deref(&self) -> &T {
        &self.symbol
    }
}

impl<'lib, T> DerefMut for Sym<'lib, T> {
    fn deref_mut(&mut self) -> &mut T {
        &mut self.symbol
    }
}

unsafe impl<'lib, T: Send> Send for Sym<'lib, T> {}
unsafe impl<'lib, T: Sync> Sync for Sym<'lib, T> {}

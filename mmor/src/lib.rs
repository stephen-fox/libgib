use std::{
    error::Error,
    path::{Path, PathBuf},
};

#[cfg(unix)]
pub mod unix;

#[cfg(windows)]
pub mod windows;

/// objects enumerates the memory-mapped objects in the current process.
///
/// It is a wrapper for the objects_with_options function.
pub unsafe fn objects() -> Result<Objects, Box<dyn Error>> {
    unsafe {
        objects_with_options(ObjectLookupOptions {
            skip_invalid_handles: false,
        })
    }
}

/// ObjectLookupOptions are options that customize the behavior of
/// the objects_with_options function.
pub struct ObjectLookupOptions {
    /// skip_invalid_handles ignores objects that return an invalid
    /// handle error on Windows when set to true.
    ///
    /// Refer to commit 052b2dd458fb588c048566491815026c614ffee8
    /// for details.
    pub skip_invalid_handles: bool,
}

/// objects_with_otions enumerates the memory-mapped objects in the
/// current process.
pub unsafe fn objects_with_options(
    options: ObjectLookupOptions,
) -> Result<Objects, Box<dyn Error>> {
    #[cfg(unix)]
    unsafe {
        unix::objects(options)
    }

    #[cfg(windows)]
    unsafe {
        windows::objects(options)
    }
}

/// Objects contains the memory-mapped objects and other details.
pub struct Objects {
    /// objects are the memory-mapped objects for a process.
    pub objects: Vec<Object>,
}

/// Object represents a single memory-mapped object.
pub struct Object {
    /// name is the name of the object, if any.
    pub name: Option<String>,

    /// path is the file path of the object, if any.
    pub path: Option<PathBuf>,

    /// addr is the memory address of the object.
    pub addr: usize,
}

impl std::fmt::Display for Object {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut path_str = String::new();
        if let Some(path) = &self.path {
            path_str = path.display().to_string();
        }

        let mut name_str = "";
        if let Some(name) = &self.name {
            name_str = name;
        }

        write!(
            f,
            "addr: {:#x} | name: '{}' | path: '{}'",
            self.addr, name_str, path_str,
        )?;

        Ok(())
    }
}

pub(crate) fn path_basename(pathbuf: &Path) -> Option<String> {
    if let Some(os_str) = pathbuf.file_name() {
        if let Some(str_ref) = os_str.to_str() {
            return Some(str_ref.to_string());
        }
    }

    None
}

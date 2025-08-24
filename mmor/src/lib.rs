use std::{
    error::Error,
    path::{Path, PathBuf},
};

#[cfg(unix)]
pub mod unix;

#[cfg(windows)]
pub mod windows;

pub struct Object {
    pub name: Option<String>,
    pub path: Option<PathBuf>,
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

pub unsafe fn objects() -> Result<Vec<Object>, Box<dyn Error>> {
    #[cfg(unix)]
    unsafe {
        unix::objects()
    }

    #[cfg(windows)]
    unsafe {
        windows::objects()
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

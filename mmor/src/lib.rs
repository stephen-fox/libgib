use std::{error::Error, path::PathBuf};

#[cfg(unix)]
pub mod unix;

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
}

use std::error::Error;

#[cfg(unix)]
pub mod unix;

pub struct Object {
    pub name: Option<String>,
    pub addr: usize,
}

struct Objects {
    objects: Vec<Object>,
}

pub unsafe fn objects() -> Result<Vec<Object>, Box<dyn Error>> {
    #[cfg(unix)]
    unsafe {
        unix::objects()
    }
}

use std::{
    io::{Read, Write},
    os::unix::net::{UnixListener, UnixStream},
    path::Path,
};

pub struct Listener(UnixListener);

impl Listener {
    pub fn bind(path: &Path) -> Result<Self, std::io::Error> {
        let unix_listener = UnixListener::bind(path)?;

        Ok(Self(unix_listener))
    }
}

impl crate::Listener for Listener {
    fn accept(&mut self) -> std::io::Result<Box<dyn crate::Conn>> {
        let stream = self.0.accept()?;

        Ok(Box::new(Conn(stream.0)))
    }
}

pub struct Conn(UnixStream);

impl Conn {
    pub fn dial(path: &Path) -> Result<Self, std::io::Error> {
        let stream = UnixStream::connect(path)?;

        Ok(Self(stream))
    }
}

impl crate::Conn for Conn {}

impl Read for Conn {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        self.0.read(buf)
    }
}

impl Write for Conn {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.0.write(buf)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.0.flush()
    }
}

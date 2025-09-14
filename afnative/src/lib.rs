use std::{
    io::{Read, Write},
    path::Path,
};

#[cfg(unix)]
pub mod unix;

#[cfg(windows)]
pub mod windows;

pub fn listen(path: &Path) -> std::io::Result<Box<dyn Listener>> {
    let result;

    #[cfg(unix)]
    {
        result = unix::Listener::bind(path)?;
    }

    #[cfg(windows)]
    {
        result = windows::PipeListener::bind(path)?;
    }

    Ok(Box::new(result))
}

pub trait Listener: Send {
    fn accept(&mut self) -> std::io::Result<Box<dyn Conn>>;
}

pub fn dial(path: &Path) -> std::io::Result<Box<dyn Conn>> {
    let result;

    #[cfg(unix)]
    {
        result = unix::Conn::dial(path)?;
    }

    #[cfg(windows)]
    {
        result = windows::PipeStream::connect(path)?;
    }

    Ok(Box::new(result))
}

pub trait Conn: Read + Write + Send {}

#[cfg(test)]
mod test {
    use super::*;

    use std::{
        fs::{create_dir_all, remove_file},
        path::PathBuf,
        thread,
    };

    macro_rules! or_panic {
        ($e:expr) => {
            match $e {
                Ok(e) => e,
                Err(e) => {
                    panic!("{}", e);
                }
            }
        };
    }

    #[test]
    fn basic() {
        let resources = test_resources("basic");

        let msg1 = b"hello";
        let msg2 = b"world!";

        let mut listener = or_panic!(listen(resources.path.as_path()));

        let thread = thread::spawn(move || {
            let mut stream = or_panic!(listener.accept());
            let mut buf = [0; 5];
            or_panic!(stream.read(&mut buf));
            assert_eq!(&msg1[..], &buf[..]);
            or_panic!(stream.write_all(msg2));
        });

        let mut stream = or_panic!(dial(resources.path.as_path()));

        or_panic!(stream.write_all(msg1));

        let mut buf = vec![];
        or_panic!(stream.read_to_end(&mut buf));
        assert_eq!(&msg2[..], &buf[..]);
        drop(stream);

        thread.join().unwrap();
    }

    #[test]
    fn iter() {
        let resources = test_resources("iter");

        let mut listener = or_panic!(listen(resources.path.as_path()));

        let thread = thread::spawn(move || {
            for _ in 0..2 {
                let mut stream = or_panic!(listener.accept());
                let mut buf = [0];
                or_panic!(stream.read(&mut buf));
            }
        });

        for _ in 0..2 {
            let mut stream = or_panic!(dial(resources.path.as_path()));
            or_panic!(stream.write_all(&[0]));
        }

        thread.join().unwrap();
    }

    struct TestResources {
        path: PathBuf,
    }

    fn test_resources(test_name: &str) -> TestResources {
        let base_dir = env!("CARGO_MANIFEST_DIR");

        if cfg!(windows) {
            let mut pb = PathBuf::from(r"\\.\pipe\");

            // Hex encode string code by Stackoverflow user han solo:
            // https://stackoverflow.com/a/62758411
            let mut s = String::new();
            use std::fmt::Write as FmtWrite; // renaming import to avoid collision
            for b in base_dir.as_bytes() {
                or_panic!(write!(s, "{:02x}", b));
            }

            pb.push(s);
            pb.push(test_name);

            TestResources { path: pb }
        } else {
            let mut pb = PathBuf::from(base_dir);

            pb.push("target");
            pb.push("tests");

            or_panic!(create_dir_all(pb.as_path()));

            pb.push(test_name);

            if pb.exists() {
                or_panic!(remove_file(pb.as_path()));
            }

            TestResources { path: pb }
        }
    }
}

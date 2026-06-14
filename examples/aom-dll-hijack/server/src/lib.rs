use std::error::Error;
use std::ffi::c_void;
use std::io::Write;
use std::path::Path;
use std::sync::mpsc;
use std::thread;
use std::thread::JoinHandle;

pub struct Server {
    pub handle: JoinHandle<()>,
    send: mpsc::Sender<MemcpyArgs>,
}

struct MemcpyArgs {
    dst: usize,
    src: usize,
    nbytes: usize,
}

pub fn start() -> Result<Server, Box<dyn Error>> {
    let socket_path = Path::new("//./pipe/aomx-poke");
    println!("{:?}", socket_path);
    let mut listener = afnative::listen(socket_path)?;
    let (send, receive) = mpsc::channel::<MemcpyArgs>();

    let jh = thread::spawn(move || {
        loop {
            let stream = listener.accept();
            if stream.is_err() {
                return;
            }

            let mut stream = stream.unwrap();

            for memcopy_arg in receive.iter() {
                if stream
                    .write_fmt(format_args!(
                        "buhbuh dst: {:#x} | src: {:#x} | size: {:#x}\n",
                        memcopy_arg.dst, memcopy_arg.src, memcopy_arg.nbytes
                    ))
                    .is_err()
                {
                    break;
                }
            }
        }
    });

    Ok(Server {
        handle: jh,
        send: send,
    })
}

impl Server {
    pub fn handle_fake_memcpy(&self, dst: *mut c_void, src: *mut c_void, nbytes: usize) {
        let _ = self.send.send(MemcpyArgs {
            dst: dst.addr(),
            src: src.addr(),
            nbytes,
        });
    }
}

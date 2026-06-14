fn main() {
    match server::start() {
        Ok(srv) => srv.handle.join().unwrap(),
        Err(err) => {
            eprintln!("fatal: {err}");
            std::process::exit(1);
        }
    }
}
use std::error::Error;

const LIBRARY_PATH: &str = {
    if cfg!(target_os = "windows") {
        "target\\debug\\deps\\example_lib.dll"
    } else {
        "target/debug/deps/libexample_lib.so"
    }
};

fn main() -> Result<(), Box<dyn Error>> {
    let lib = unsafe { dlrkit::Dl::open(Some(LIBRARY_PATH))? };

    let add: fn(left: u64, right: u64) -> u64 = unsafe { lib.sym("add")? };

    eprintln!("add result: {}", add(62, 7));

    Ok(())
}

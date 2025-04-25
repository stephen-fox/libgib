use std::error::Error;

fn main() -> Result<(), Box<dyn Error>>{
    let lib = unsafe {dlrkit::Dl::open(Some("examples\\example-lib\\target\\debug\\example_lib.dll"))?};

    let add: fn(left: u64, right: u64) -> u64 = unsafe { lib.sym("Add")? };

    eprintln!("add result: {}", add(62, 7));

    Ok(())
}

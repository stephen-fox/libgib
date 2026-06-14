mod buildtools;

fn main() {
    if let Err(err) = main_with_error() {
        eprintln!("fatal: {err}");
        std::process::exit(1);
    };
}

fn main_with_error() -> Result<(), Box<dyn std::error::Error>> {
    buildtools::on_build()
}

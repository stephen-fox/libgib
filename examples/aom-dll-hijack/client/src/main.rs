use std::io::{BufRead, BufReader};
use std::path::Path;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let socket_path = Path::new("//./pipe/aomx-poke");
    println!("{:?}", socket_path);

    let stream = afnative::dial(socket_path)?;

    let reader = BufReader::new(stream);
    for line in reader.lines() {
        let line = line?;
        println!("{}", line);
    }

    Ok(())
}

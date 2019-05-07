use std::io::{BufRead, BufReader};
use std::net::{TcpListener};

fn main() -> std::io::Result<()> {
    let listener = TcpListener::bind("localhost::6010")?;
    let (stream, _) = listener.accept()?;

    let mut reader = BufReader::new(stream);
    let mut message = String::new();
    let _ = reader.read_line(&mut message)?;

    println!("{}", message);

    Ok(())
}

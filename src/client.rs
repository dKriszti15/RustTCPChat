use std::io::{self, Write, Read};
use std::net::TcpStream;
use std::thread;

fn main() -> io::Result<()> {
    let host = "localhost";
    let port = 1234;
    let address = format!("{}:{}", host, port);
    println!("The address looks like: {}", address);

    match TcpStream::connect(address) {
        Ok(mut stream) => {
            let mut username = String::new();

            print!("Your username: ");
            let _ = io::stdout().flush();

            // read username from console
            match io::stdin().read_line(&mut username) {
                Ok(_) => {}
                Err(error) => {
                    println!("error: {}", error);
                    return Ok(());
                }
            }

            // username -> server
            stream.write_all(username.trim().as_bytes()).expect("Failed to write username to server");

            let mut buffer = vec![0; 1024];
            stream.read(&mut buffer).expect("Failed to read from server");
            let response = String::from_utf8_lossy(&buffer).trim_end_matches('\0').trim().to_string();
            println!("Received response from server: {}", response);

            if response == "username unavailable" {
                eprintln!("Username is unavailable, exiting...");
                return Ok(());
            }

            // thread <- messages
            let mut stream_clone = stream.try_clone().expect("Failed to clone stream");
            thread::spawn(move || loop {
                let mut buffer = vec![0; 1024];
                match stream_clone.read(&mut buffer) {
                    Ok(0) => break, // disconnected
                    Ok(_) => {
                        let msg = String::from_utf8_lossy(&buffer).trim_end_matches('\0').trim().to_string();
                        println!("\n{}\n", msg);
                    }
                    Err(e) => {
                        eprintln!("Failed to read from server: {}", e);
                        break;
                    }
                }
            });

            loop {
                let mut message = String::new();

                print!("> ");
                let _ = io::stdout().flush();

                match io::stdin().read_line(&mut message) {
                    Ok(_) => {
                        let trimmed_message = message.trim();

                        if trimmed_message == "disconnect" {
                            stream.write_all(trimmed_message.as_bytes()).expect("Failed to write to server");
                            println!("Disconnected.");
                            break;
                        } else if trimmed_message.starts_with("/") {
                            stream.write_all(trimmed_message.as_bytes()).expect("Failed to write to server");
                        } else {
                            stream.write_all(trimmed_message.as_bytes()).expect("Failed to write to server");
                        }
                    }
                    Err(error) => {
                        println!("error: {}", error);
                        break;
                    }
                }
            }
        }
        Err(e) => {
            eprintln!("Failed to connect: {}", e);
        }
    }

    Ok(())
}

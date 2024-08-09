use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex};
use std::thread;

#[derive(Debug)]
struct Client {
    username: String,
    stream: TcpStream,
}

impl Client {
    fn new(username: String, stream: TcpStream) -> Self {
        Client { username, stream }
    }
}

type Clients = Arc<Mutex<Vec<Client>>>;

fn add_client(clients: &Clients, client: Client) {
    let mut clients = clients.lock().unwrap();
    clients.push(client);
    println!("Clients connected: {}", clients.len());
    for cl in clients.iter() {
        println!("{}", cl.username);
    }
}

fn remove_client(clients: &Clients, username: &str) {
    let mut clients = clients.lock().unwrap();
    clients.retain(|client| client.username != username);
    println!("Clients connected: {}", clients.len());
}

fn username_unavailable(clients: &Clients, username: &str) -> bool {
    let clients = clients.lock().unwrap();
    clients.iter().any(|client| client.username == username)
}

fn get_user_stream(clients: &Clients, username: &str) -> Option<TcpStream> {
    let clients = clients.lock().unwrap();
    for cl in clients.iter() {
        if cl.username.trim_end_matches('\0').eq(username) {
            return Some(cl.stream.try_clone().expect("Clone error"));
        }
    }
    None
}

fn broadcast_message(clients: &Clients, message: &str, sender: &str) {
    let mut clients = clients.lock().unwrap();
    for client in clients.iter_mut() {
        if client.username != sender {
            let _ = client.stream.write(message.as_bytes());
        }
    }
}

fn handle_client(mut stream: TcpStream, clients: Clients) {
    
    let mut username_buffer = [0; 50];

    if let Err(e) = stream.read(&mut username_buffer) {
        eprintln!("Failed to read username from client: {}", e);
        return;
    }

    let username = String::from_utf8_lossy(&username_buffer).trim_end_matches('\0').trim().to_string();

    if username_unavailable(&clients, &username) {
        eprintln!("Username '{}' already taken.", username);
        let _ = stream.write(b"username unavailable\n");
        return;
    } else {
        add_client(&clients, Client::new(username.clone(), stream.try_clone().unwrap()));

        let response = format!("Hello, {}!\n", username);
        if let Err(e) = stream.write(response.as_bytes()) {
            eprintln!("Failed to send greeting to {}: {}", username, e);
            return;
        }
    }

    loop {
        let mut buffer = [0; 1024];
        match stream.read(&mut buffer) {
            Ok(_) => {
                let message = String::from_utf8_lossy(&buffer[..]).trim_end_matches('\0').trim().to_string();
                println!("Received a message from {}: {}.", username, message);

                if message.starts_with("@") {
                    // msg -> specific user
                    let mut parts = message.splitn(2, ' ');
                    let target_username = parts.next().unwrap().trim_start_matches('@');
                    let msg = parts.next().unwrap_or("");

                    if let Some(mut target_stream) = get_user_stream(&clients, target_username) {
                        let msg_to_send = format!("*pm from {}: {}", username, msg);
                        let _ = target_stream.write(msg_to_send.as_bytes());
                    } else {
                        let _ = stream.write(b"User not found\n");
                    }
                } else if message == "disconnect" {
                    println!("Client {} disconnected.", username);
                    remove_client(&clients, &username);
                    break;
                } else if message.starts_with("/") {
                    // list connected users
                    let clients = clients.lock().unwrap();
                    let usernames: Vec<String> = clients.iter().map(|client| client.username.clone()).collect();
                    let msg_to_send = format!("Connected clients: {}\n", usernames.join(", "));
                    let _ = stream.write(msg_to_send.as_bytes());
                } else {
                    // message -> all users
                    let msg_to_send = format!("{}: {}\n", username, message);
                    broadcast_message(&clients, &msg_to_send, &username);
                }
            }
            Err(e) => {
                eprintln!("Failed to read message from {}: {}", username, e);
                remove_client(&clients, &username);
                break;
            }
        }
    }
}

fn main() -> std::io::Result<()> {
    let host = "localhost";
    let port = 1234;
    let clients: Clients = Arc::new(Mutex::new(Vec::new()));

    let listener = TcpListener::bind((host, port))?;
    println!("Server running on {}:{}", host, port);

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                let clients = Arc::clone(&clients);
                thread::spawn(|| handle_client(stream, clients));
            }
            Err(e) => eprintln!("Failed to accept connection: {}", e),
        }
    }

    Ok(())
}

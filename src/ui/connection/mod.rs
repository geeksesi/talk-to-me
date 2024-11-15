use std::io::{Read, Write};
use std::net::TcpStream;
use std::sync::mpsc::Sender;
use std::thread;

pub struct Connection {
    stream: TcpStream,
}

impl Connection {
    pub fn new() -> Result<Self, std::io::Error> {
        let stream = TcpStream::connect("127.0.0.1:3000")?;
        stream.set_nonblocking(true)?;
        Ok(Connection { stream })
    }

    pub fn start_listening(&mut self, response_tx: Sender<String>) {
        let mut stream = self.stream.try_clone().expect("Failed to clone stream");
        
        thread::spawn(move || {
            let mut buffer = [0; 1024];
            
            loop {
                match stream.read(&mut buffer) {
                    Ok(n) if n > 0 => {
                        let response = String::from_utf8_lossy(&buffer[..n]).to_string();
                        let _ = response_tx.send(response);
                    }
                    Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                        std::thread::sleep(std::time::Duration::from_millis(100));
                        continue;
                    }
                    _ => break,
                }
            }
        });
    }

    pub fn send_message(&mut self, message: String) -> Result<(), std::io::Error> {
        self.stream.write_all(message.as_bytes())?;
        Ok(())
    }
} 
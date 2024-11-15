use std::sync::mpsc::{channel, Sender, Receiver};
use std::thread;
use crate::ui::connection::Connection;

pub struct WindowConnection {
    sender: Sender<String>,
    receiver: Receiver<String>,
}

impl WindowConnection {
    pub fn new() -> Self {
        let (tx, rx) = channel();
        let (response_tx, response_rx) = channel();
        
        thread::spawn(move || {
            if let Ok(mut connection) = Connection::new() {
                connection.start_listening(response_tx);
                
                for message in rx {
                    let _ = connection.send_message(message);
                }
            }
        });

        WindowConnection {
            sender: tx,
            receiver: response_rx,
        }
    }

    pub fn send(&self, message: String) {
        let _ = self.sender.send(message);
    }

    pub fn try_receive(&self) -> Option<String> {
        self.receiver.try_recv().ok()
    }
} 
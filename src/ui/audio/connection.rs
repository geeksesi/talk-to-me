use tokio::net::UdpSocket;
use std::sync::Arc;

const MAX_UDP_PACKET_SIZE: usize = 1200; // Conservative size to avoid fragmentation

#[derive(Clone)]
pub struct AudioConnection {
    socket: Arc<UdpSocket>,
}

impl AudioConnection {
    pub async fn new() -> std::io::Result<Self> {
        let socket = UdpSocket::bind("0.0.0.0:0").await?;
        socket.connect("127.0.0.1:3001").await?;
        
        Ok(AudioConnection { 
            socket: Arc::new(socket)
        })
    }

    pub async fn send_audio(&self, data: &[u8]) -> std::io::Result<()> {
        // Split data into chunks and send each chunk
        for chunk in data.chunks(MAX_UDP_PACKET_SIZE) {
            self.socket.send(chunk).await?;
        }
        Ok(())
    }
} 
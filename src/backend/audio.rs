use std::collections::HashMap;
use std::net::SocketAddr;
use std::time::{Duration, Instant};
use tokio_uring::net::UdpSocket;
use std::sync::Arc;

const MAX_UDP_PACKET_SIZE: usize = 1200; // Conservative size to avoid fragmentation

pub struct AudioChunk {
    data: Vec<u8>,
    last_update: Instant,
}

pub struct AudioProcessor {
    chunks: HashMap<SocketAddr, AudioChunk>,
    socket: Arc<UdpSocket>,
}

impl AudioProcessor {
    pub fn new(socket: Arc<UdpSocket>) -> Self {
        Self {
            chunks: HashMap::new(),
            socket,
        }
    }

    pub async fn process_packet(&mut self, addr: SocketAddr, data: Vec<u8>) -> miette::Result<()> {
        // Store or update chunk
        let chunk = self.chunks.entry(addr).or_insert_with(|| AudioChunk {
            data: Vec::new(),
            last_update: Instant::now(),
        });
        
        chunk.data.extend(data.iter());
        chunk.last_update = Instant::now();

        tracing::debug!("Received audio chunk of size {} from {}", data.len(), addr);
        
        // // Echo back in smaller chunks if needed
        // for echo_chunk in data.chunks(MAX_UDP_PACKET_SIZE) {
        //     self.socket.send_to(echo_chunk, addr).await.into_diagnostic()?;
        // }
        Ok(())
    }

    pub fn cleanup_old_chunks(&mut self) {
        const MAX_AGE: Duration = Duration::from_secs(30);
        self.chunks.retain(|_, chunk| {
            chunk.last_update.elapsed() < MAX_AGE
        });
    }
} 
use std::net::SocketAddr;
use miette::IntoDiagnostic;
use tokio_uring::net::UdpSocket;
use std::sync::Arc;
use super::audio::AudioProcessor;

pub struct UdpHandler {
    socket: Arc<UdpSocket>,
    audio_processor: AudioProcessor,
}

impl UdpHandler {
    pub async fn new(addr: &str) -> miette::Result<Self> {
        let udp_addr: SocketAddr = addr.parse().into_diagnostic()?;
        tracing::info!("Attempting to bind UDP socket to {}", udp_addr);
        let socket = Arc::new(UdpSocket::bind(udp_addr).await.into_diagnostic()?);
        tracing::info!("Successfully bound UDP socket to {}", udp_addr);
        
        Ok(Self {
            socket: Arc::clone(&socket),
            audio_processor: AudioProcessor::new(socket),
        })
    }

    pub async fn process_packet(&mut self, data: Vec<u8>, addr: SocketAddr) -> miette::Result<()> {
        tracing::info!("Received audio chunk from {}", addr.clone());

        self.audio_processor.process_packet(addr, data).await
    }

    pub fn get_socket(&self) -> Arc<UdpSocket> {
        Arc::clone(&self.socket)
    }
} 
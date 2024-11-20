mod backend;

use miette::IntoDiagnostic;
use r3bl_terminal_async::port_availability;
use std::net::SocketAddr;
use tokio::task::AbortHandle;
use tokio_uring::net::{TcpListener, UdpSocket};
use tokio_util::sync::CancellationToken;
use std::sync::Arc;

use backend::ConnectionHandler;

async fn process_socket_connection(stream: tokio_uring::net::TcpStream) -> miette::Result<()> {
    let mut handler = ConnectionHandler::new(stream);
    handler.process().await
}

async fn process_audio_packet(
    socket: Arc<UdpSocket>,
    client_addr: SocketAddr,
    size: usize,
) -> miette::Result<()> {
    let mut buf = vec![0u8; size];
    // socket.send_to(&buf, client_addr).await.into_diagnostic()?;
    Ok(())
}

async fn start_server(cancellation_token: CancellationToken) -> miette::Result<()> {
    let tcp_listener = {
        let tcp_addr: SocketAddr = "0.0.0.0:3000".parse().into_diagnostic()?;

        match port_availability::check(tcp_addr).await? {
            port_availability::Status::Free => {
                println!("Port {} is available", tcp_addr.port());
            }
            port_availability::Status::Occupied => {
                println!("Port {} is not available", tcp_addr.port());
            }
        }

        TcpListener::bind(tcp_addr).into_diagnostic()?
    };

    let udp_socket = Arc::new({
        let udp_addr: SocketAddr = "0.0.0.0:3001".parse().into_diagnostic()?;
        match port_availability::check(udp_addr).await? {
            port_availability::Status::Free => {
                println!("Port {} is available", udp_addr.port());
            }
            port_availability::Status::Occupied => {
                println!("Port {} is not available", udp_addr.port());
            }
        }
        let socket = UdpSocket::bind(udp_addr).await.into_diagnostic()?;
        socket.connect(udp_addr).await.into_diagnostic()?;
        socket
    });

    tracing::info!("TCP Listening on 3000");
    tracing::info!("UDP Listening on 3001");

    let mut abort_handles: Vec<AbortHandle> = Vec::new();
    let mut buf = vec![0u8; 1024];

    loop {
        tokio::select! {
            _ = cancellation_token.cancelled() => {
                tracing::info!("Cancellation token received, shutting down");
                abort_handles.iter().for_each(|handle| handle.abort());
                break;
            }
            result_tcp_stream = tcp_listener.accept() => {
                let (tcp_stream, _) = result_tcp_stream.into_diagnostic()?;
                let join_handle = tokio_uring::spawn(process_socket_connection(tcp_stream));
                abort_handles.push(join_handle.abort_handle());
            }
            (result_udp, buf) = udp_socket.recv_from(buf) => {
                let (size, addr) = result_udp.into_diagnostic()?;
                let socket = Arc::clone(&udp_socket);
                let join_handle = tokio_uring::spawn(process_audio_packet(socket, addr, size));
                abort_handles.push(join_handle.abort_handle());
            }
        }
    }

    Ok(())
}

fn main() -> miette::Result<()> {
    dotenv::dotenv().ok();
    register_tracing_subscriber();

    let cancellation_token = tokio_util::sync::CancellationToken::new();
    let cancellation_token_clone = cancellation_token.clone();

    ctrlc::set_handler(move || {
        cancellation_token_clone.cancel();
    })
    .into_diagnostic()?;

    tokio_uring::start(start_server(cancellation_token.clone()))?;

    Ok(())
}

fn register_tracing_subscriber() {
    tracing_subscriber::fmt()
        .without_time()
        .compact()
        .with_target(true)
        .with_line_number(true)
        .with_thread_names(true)
        .init();
}

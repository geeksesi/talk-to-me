mod backend;

use miette::IntoDiagnostic;
use r3bl_terminal_async::port_availability;
use std::net::SocketAddr;
use tokio::task::AbortHandle;
use tokio_uring::net::TcpListener;
use tokio_util::sync::CancellationToken;
use backend::{ConnectionHandler, UdpHandler};
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing_subscriber::fmt::format::FmtSpan;

async fn process_socket_connection(stream: tokio_uring::net::TcpStream) -> miette::Result<()> {
    let mut handler = ConnectionHandler::new(stream);
    handler.process().await
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

    let udp_handler = Arc::new(Mutex::new(
        UdpHandler::new("0.0.0.0:3001").await?
    ));
    let udp_socket = udp_handler.lock().await.get_socket();

    tracing::info!("TCP Listening on 3000");
    tracing::info!("UDP Listening on 3001");

    let mut abort_handles: Vec<AbortHandle> = Vec::new();

    loop {
        let mut buf = vec![0u8; 1024];

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
            result = udp_socket.recv_from(buf) => {
                tracing::info!("UDP socket received something...");
                let (result, received_buf) = result;
                match result {
                    Ok((size, addr)) => {
                        tracing::info!("UDP packet received successfully: size={}, from={}", size, addr);
                        let data = received_buf[..size].to_vec();
                        let handler = Arc::clone(&udp_handler);
                        
                        let span = tracing::span!(
                            tracing::Level::INFO,
                            "udp_packet",
                            size = data.len(),
                            addr = %addr
                        );
                        
                        let join_handle = tokio_uring::spawn(async move {
                            let _enter = span.enter();
                            tracing::info!("Starting UDP packet processing");
                            let mut handler = handler.lock().await;
                            let result = handler.process_packet(data, addr).await;
                            tracing::info!("Finished UDP packet processing");
                            result
                        });
                        
                        abort_handles.push(join_handle.abort_handle());
                    }
                    Err(e) => {
                        tracing::error!("Error receiving UDP packet: {}", e);
                    }
                }
            }
        }
    }

    Ok(())
}

fn register_tracing_subscriber() {
    let subscriber = tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .with_span_events(FmtSpan::FULL)
        .with_thread_ids(true)
        .with_thread_names(true)
        .with_target(true)
        .with_line_number(true)
        .with_file(true)
        .compact()
        .without_time();

    tracing::subscriber::set_global_default(subscriber.finish())
        .expect("Failed to set tracing subscriber");
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

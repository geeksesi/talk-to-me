use std::net::SocketAddr;

use miette::IntoDiagnostic;
use r3bl_terminal_async::port_availability;

use tokio::task::AbortHandle;
use tokio_uring::{
    buf::BoundedBuf,
    net::{TcpListener, TcpStream},
};
use tokio_util::sync::CancellationToken;

async fn start_server(cancellation_token: CancellationToken) -> miette::Result<()> {
    let tcp_listener = {
        let addr: SocketAddr = "0.0.0.0:3000".parse().into_diagnostic()?;

        match port_availability::check(addr).await? {
            port_availability::Status::Free => {
                println!("Port {} is available", addr.port());
            }
            port_availability::Status::Occupied => {
                println!("Port {} is not available", addr.port());
            }
        }

        TcpListener::bind(addr).into_diagnostic()?
    };

    tracing::info!("TCP Listening on {}", "3000");

    let mut abort_handles: Vec<AbortHandle> = Vec::new();

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

async fn process_socket_connection(stream: TcpStream) -> miette::Result<()> {
    // socket.ready(b"Hello, world!\n").await;
    tracing::info!("Processing socket connection");
    let mut total_byte_read = 0;
    let mut buf = vec![0u8; 10];
    loop {
        let (result_num_bytes_read, return_buf) = stream.read(buf).await;
        buf = return_buf;
        let num_bytes_read = result_num_bytes_read.into_diagnostic()?;
        // check for EOF
        if num_bytes_read == 0 {
            break;
        }
        let (result_num_byte_written, slice) = stream.write_all(buf.slice(..num_bytes_read)).await;
        result_num_byte_written.into_diagnostic()?;
        buf = slice.into_inner();
        total_byte_read += num_bytes_read;

        tracing::info!("total_byte_read: {}", total_byte_read);
    }
    tracing::info!("connection is done");

    Ok(())
}

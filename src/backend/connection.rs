use miette::IntoDiagnostic;
use tokio_uring::net::TcpStream;

pub struct ConnectionHandler {
    stream: TcpStream,
    total_bytes_read: usize,
    buffer: Vec<u8>,
}

impl ConnectionHandler {
    pub fn new(stream: TcpStream) -> Self {
        Self {
            stream,
            total_bytes_read: 0,
            buffer: vec![0u8; 1024],
        }
    }

    pub async fn process(&mut self) -> miette::Result<()> {
        tracing::info!("Processing socket connection");

        loop {
            let (result_num_bytes_read, return_buf) = self.stream.read(self.buffer.clone()).await;
            self.buffer = return_buf;
            let num_bytes_read = result_num_bytes_read.into_diagnostic()?;
            
            if num_bytes_read == 0 {
                break;
            }

            let response = self.create_response(&self.buffer[..num_bytes_read], num_bytes_read);
            let response_bytes = response.into_bytes();
            let (result_num_byte_written, _) = self.stream.write_all(response_bytes).await;
            result_num_byte_written.into_diagnostic()?;
            
            self.total_bytes_read += num_bytes_read;
            tracing::info!("total_byte_read: {}", self.total_bytes_read);
        }
        tracing::info!("connection is done");

        Ok(())
    }

    fn create_response(&self, message: &[u8], length: usize) -> String {
        format!(
            "**Received your message:**\n\
            ```\n{}\n```\n\n\
            Here's a sample response:\n\n\
            # Lorem Ipsum\n\
            ## About this text\n\
            Lorem ipsum dolor sit amet, *consectetur* adipiscing elit. \
            Sed do **eiusmod** tempor incididunt ut labore et dolore magna aliqua.\n\n\
            - Point 1\n\
            - Point 2\n\
            - Point 3\n\n\
            > This is a blockquote with your message length: {} bytes\n",
            String::from_utf8_lossy(message),
            length
        )
    }
} 
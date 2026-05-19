//! Non-blocking I/O wrapper for TCP socket operations.
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    net::{
        TcpStream,
        tcp::{OwnedReadHalf, OwnedWriteHalf},
    },
};

/// Manages buffered reading and writing on a TCP socket split into independent halves.
///
/// Provides async line-based I/O for XSCP protocol messages, which are newline-delimited.
pub struct SocketIo {
    reader: BufReader<OwnedReadHalf>,
    writer: OwnedWriteHalf,
}

impl SocketIo {
    /// Creates a new [`SocketIo`] from a connected TCP socket.
    ///
    /// Splits the socket into independent read and write halves to enable
    /// concurrent operations in async contexts.
    pub fn new(socket: TcpStream) -> Self {
        let (reader, writer) = socket.into_split();
        Self {
            reader: BufReader::new(reader),
            writer,
        }
    }

    /// Reads a line from the socket until a newline character.
    ///
    /// # Returns
    ///
    /// - `Ok(Some(line))` — successfully read a complete line (including the trailing newline)
    /// - `Ok(None)` — socket closed (EOF reached, 0 bytes read)
    /// - `Err(_)` — I/O error during reading
    pub async fn read(
        &mut self,
    ) -> Result<Option<String>, Box<dyn std::error::Error + Send + Sync>> {
        let mut buf = String::new();
        match self.reader.read_line(&mut buf).await {
            Ok(0) => Ok(None),
            Ok(_) => Ok(Some(buf)),
            Err(err) => Err(Box::new(err)),
        }
    }

    /// Writes data to the socket.
    ///
    /// # Arguments
    ///
    /// - `data` — the string to write (typically an XSCP PDU)
    pub async fn write(&mut self, data: &str) -> std::io::Result<()> {
        self.writer.write_all(data.as_bytes()).await
    }
}

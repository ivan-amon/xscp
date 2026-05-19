use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    net::{TcpStream, tcp::{OwnedReadHalf, OwnedWriteHalf}},
};

pub struct SocketIo {
    reader: BufReader<OwnedReadHalf>,
    writer: OwnedWriteHalf,
}

impl SocketIo {
    pub fn new(socket: TcpStream) -> Self {
        let (reader, writer) = socket.into_split();
        Self {
            reader: BufReader::new(reader),
            writer,
        }
    }

    pub async fn read(&mut self) -> Result<Option<String>, Box<dyn std::error::Error + Send + Sync>> {
        let mut buf = String::new();
        match self.reader.read_line(&mut buf).await {
            Ok(0) => Ok(None),
            Ok(_) => Ok(Some(buf)),
            Err(err) => Err(Box::new(err)),
        }
    }

    pub async fn write(&mut self, data: &str) -> std::io::Result<()> {
        self.writer.write_all(data.as_bytes()).await
    }
}

use super::{OUTPUT_BYTES, Observer, invalid};
use futures::future::BoxFuture;
use rmcp::model::ServerJsonRpcMessage;
use std::{
    io::{self, Write},
    sync::Arc,
    time::Duration,
};
use tokio::io::{AsyncWrite, AsyncWriteExt};

struct Output(Vec<u8>);
impl Write for Output {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        if self
            .0
            .len()
            .checked_add(bytes.len())
            .is_none_or(|n| n > OUTPUT_BYTES)
        {
            return Err(invalid("response byte budget"));
        }
        self.0.extend_from_slice(bytes);
        Ok(bytes.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
pub(super) fn write<W: AsyncWrite + Unpin + Send + 'static>(
    writer: Arc<tokio::sync::Mutex<W>>,
    observer: Arc<Observer>,
    message: ServerJsonRpcMessage,
) -> BoxFuture<'static, io::Result<()>> {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(10);
    Box::pin(async move {
        let mut output = Output(Vec::new());
        if serde_json::to_writer(&mut output, &message).is_err() {
            observer.fail("response byte budget or encoding");
            return Err(invalid("response encoding"));
        }
        output.0.push(b'\n');
        let write = async {
            let mut writer = writer.lock().await;
            writer.write_all(&output.0).await?;
            writer.flush().await
        };
        match tokio::time::timeout_at(deadline, write).await {
            Ok(Ok(())) => Ok(()),
            _ => {
                observer.fail("response write failed or timed out");
                Err(invalid("response write"))
            }
        }
    })
}

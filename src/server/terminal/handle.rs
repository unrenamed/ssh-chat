use log::{error, trace};
use russh::{server::Handle, ChannelId};

#[derive(Clone)]
pub struct TerminalHandle {
    handle: Handle,
    sink: Vec<u8>, // The sink collects the data which is finally flushed to the handle.
    channel_id: ChannelId,
    closed: bool,
}

impl TerminalHandle {
    pub fn new(channel_id: ChannelId, handle: Handle) -> Self {
        Self {
            channel_id,
            handle,
            sink: Vec::new(),
            closed: false,
        }
    }

    pub fn close(&mut self) {
        let handle = self.handle.clone();
        let channel_id = self.channel_id.clone();

        tokio::spawn(async move {
            let result = handle.close(channel_id).await;
            if result.is_err() {
                error!(
                    "[channel {}] Failed to close session: {:?}",
                    channel_id, result
                );
            }
        });

        self.closed = true;
    }
}

// The crossterm backend writes to the terminal handle.
impl std::io::Write for TerminalHandle {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.sink.extend_from_slice(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        if self.closed {
            trace!(
                "[channel {}] Handle is already closed. Ignoring this flush call",
                self.channel_id
            );
            return Ok(());
        }

        let handle = self.handle.clone();
        let channel_id = self.channel_id;
        let data = self.sink.clone().into();
        futures::executor::block_on(async move {
            let result = handle.data(channel_id, data).await;
            if result.is_err() {
                error!(
                    "[channel {}] Failed to send data to the handle: {:?}",
                    channel_id, result
                );
            }
        });

        self.sink.clear();
        Ok(())
    }
}

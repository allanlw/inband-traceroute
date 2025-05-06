use anyhow::Context;
use libc::{IPPROTO_RAW, SOCK_RAW};
use socket2::{Domain, SockAddr, Socket};
use tokio::io::unix::AsyncFd;

#[derive(Debug)]
pub struct AsyncWriteOnlyIPRawSocket {
    inner: AsyncFd<Socket>,
}

/// Simplest possible async socket wrapper for sending raw IP packets.
impl AsyncWriteOnlyIPRawSocket {
    pub fn new(domain: Domain) -> anyhow::Result<Self> {
        // Note: IP_HDRINCL is implied when protocol=IPPROTO_RAW see raw(7)
        let socket = Socket::new(domain, SOCK_RAW.into(), Some(IPPROTO_RAW.into()))
            .context("failed to create socket")?;
        socket
            .set_nonblocking(true)
            .context("Failed to set nonblocking")?;
        let async_fd = AsyncFd::new(socket)?;
        Ok(AsyncWriteOnlyIPRawSocket { inner: async_fd })
    }

    pub async fn send_to(&self, buf: &[u8], addr: &SockAddr) -> anyhow::Result<usize> {
        loop {
            let mut guard = self.inner.writable().await?;

            match guard.try_io(|inner| inner.get_ref().send_to(buf, addr)) {
                Ok(result) => return result.context("Error from send_to"),
                Err(_would_block) => continue,
            }
        }
    }
}

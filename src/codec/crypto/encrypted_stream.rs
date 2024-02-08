use std::pin::Pin;
use std::task::{Context, Poll};

use chacha20poly1305::aead::AeadInPlace;
use chacha20poly1305::{KeyInit, XChaCha20Poly1305};
use futures::FutureExt;
use futures::{AsyncWrite, AsyncWriteExt};

use crate::codec::crypto::{AccessKey, Nonce};

pub struct EncryptingWriter<W: AsyncWrite> {
    inner: W,
    aead: XChaCha20Poly1305,
    nonce: Nonce,
}

impl<W: AsyncWrite + Unpin> EncryptingWriter<W> {
    pub fn new(inner: W, access_key: AccessKey, nonce: Nonce) -> Self {
        let aead = XChaCha20Poly1305::new(access_key.chacha_key());
        EncryptingWriter { inner, aead, nonce }
    }

    pub async fn encrypt_and_write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let mut buf_enc = buf.to_vec();

        self.aead
            .encrypt_in_place(&self.nonce, b"", &mut buf_enc)
            .expect("encryption failure");

        self.inner.write_all(&buf_enc).await?;

        Ok(buf.len())
    }
}

impl<W: AsyncWrite + Unpin + Send> AsyncWrite for EncryptingWriter<W> {
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<std::io::Result<usize>> {
        Box::pin(self.encrypt_and_write(buf)).poll_unpin(cx)
    }

    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        Pin::new(&mut self.inner).poll_flush(cx)
    }

    fn poll_close(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        Pin::new(&mut self.inner).poll_close(cx)
    }
}

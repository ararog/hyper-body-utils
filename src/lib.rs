#[cfg(feature = "http3")]
use bytes::Buf;
#[cfg(feature = "http3")]
use futures::ready;

use bytes::{Bytes};
use futures::{stream, FutureExt, Stream, TryStreamExt};
use std::pin::Pin;
use std::task::{Context, Poll};

#[cfg(feature = "http3")]
use h3::client::RequestStream as ClientRequestStream;
#[cfg(feature = "http3")]
use h3::server::RequestStream as ServerRequestStream;
#[cfg(feature = "http3")]
use h3_quinn::RecvStream;

pub use http_body_util::BodyExt;

use http_body_util::{combinators::BoxBody, StreamBody};
use hyper::body::{Body, Frame, Incoming};

#[cfg(feature = "smol-rt")]
use smol::fs::File;
#[cfg(feature = "smol-rt")]
use smol::io::AsyncReadExt;

#[cfg(feature = "tokio-rt")]
use tokio::fs::File;

#[cfg(test)]
mod tests;

pub enum HttpBody {
    Incoming(Incoming),
    Stream(BoxBody<Bytes, std::io::Error>),
    #[cfg(feature = "http3")]
    QuicClientIncoming(ClientRequestStream<RecvStream, Bytes>),
    #[cfg(feature = "http3")]
    QuicServerIncoming(ServerRequestStream<RecvStream, Bytes>),
}

impl HttpBody {
    pub fn from_incoming(incoming: Incoming) -> Self {
        HttpBody::Incoming(incoming)
    }

    #[cfg(feature = "http3")]
    pub fn from_quic_client(stream: ClientRequestStream<RecvStream, Bytes>) -> Self {
        HttpBody::QuicClientIncoming(stream)
    }

    #[cfg(feature = "http3")]
    pub fn from_quic_server(stream: ServerRequestStream<RecvStream, Bytes>) -> Self {
        HttpBody::QuicServerIncoming(stream)
    }

    pub fn from_text(text: &str) -> Self {
        Self::from_bytes(text.as_bytes())
    }

    pub fn from_file(file: File) -> Self {
        #[cfg(feature = "tokio-rt")]
        {
            let content = tokio_util::io::ReaderStream::new(file).map_ok(Frame::data);
            let body = StreamBody::new(content);
            HttpBody::Stream(BodyExt::boxed(body))
        }

        #[cfg(feature = "smol-rt")]
        {
            let content = file
                .bytes()
                .map_ok(|data| Frame::data(Bytes::copy_from_slice(&[data])));
            let body = StreamBody::new(content);
            HttpBody::Stream(BodyExt::boxed(body))
        }
    }

    pub fn from_bytes(bytes: &[u8]) -> Self {
        #[cfg(feature = "tokio-rt")]
        {
            let all_bytes = Bytes::copy_from_slice(bytes);
            let content = stream::iter(vec![Ok(all_bytes)]).map_ok(Frame::data);
            let body = StreamBody::new(content);
            HttpBody::Stream(BodyExt::boxed(body))
        }

        #[cfg(feature = "smol-rt")]
        {
            let all_bytes = Bytes::copy_from_slice(bytes);
            let content = stream::iter(vec![Ok(all_bytes)]).map_ok(Frame::data);
            let body = StreamBody::new(content);
            HttpBody::Stream(BodyExt::boxed(body))
        }
    }
}

impl Body for HttpBody {
    type Data = Bytes;

    type Error = std::io::Error;

    fn poll_frame(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Option<Result<Frame<Self::Data>, Self::Error>>> {
        match self.get_mut() {
            HttpBody::Incoming(incoming) => incoming
                .frame()
                .poll_unpin(cx)
                .map_err(std::io::Error::other),
            HttpBody::Stream(stream) => {
                stream.frame().poll_unpin(cx).map_err(std::io::Error::other)
            }
            #[cfg(feature = "http3")]
            HttpBody::QuicClientIncoming(stream) => match ready!(stream.poll_recv_data(cx)) {
                Ok(frame) => match frame {
                    Some(mut frame) => Poll::Ready(Some(Ok(Frame::data(
                        frame.copy_to_bytes(frame.remaining()),
                    )))),
                    None => {
                        cx.waker().wake_by_ref();
                        Poll::Ready(None)
                    }
                },
                Err(e) => {
                    println!("Error polling frame: {}", e);
                    Poll::Ready(Some(Err(std::io::Error::other(e))))
                }
            },
            #[cfg(feature = "http3")]
            HttpBody::QuicServerIncoming(stream) => match ready!(stream.poll_recv_data(cx)) {
                Ok(frame) => match frame {
                    Some(mut frame) => Poll::Ready(Some(Ok(Frame::data(
                        frame.copy_to_bytes(frame.remaining()),
                    )))),
                    None => {
                        cx.waker().wake_by_ref();
                        Poll::Ready(None)
                    }
                },
                Err(e) => Poll::Ready(Some(Err(std::io::Error::other(e)))),
            },
        }
    }
}

impl Stream for HttpBody {
    type Item = Result<Frame<Bytes>, std::io::Error>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.poll_frame(cx)
    }
}

/*
pub struct FileStream {
    file: OwnedMutexGuard<File>,
}

impl FileStream {
    pub fn new(file: OwnedMutexGuard<File>) -> Self {
        FileStream { file }
    }
}

impl Stream for FileStream {
    type Item = Result<Frame<Bytes>, std::io::Error>;

    #[hotpath::measure]
    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let mut me = self.as_mut();
        let mut buf = BytesMut::with_capacity(70);
        let mut read_buf = Box::pin(me.file.read_buf(&mut buf));
        match read_buf.poll_unpin(cx) {
            Poll::Ready(value) => match value {
                Ok(size) => {
                    if size == 0 {
                        Poll::Ready(None)
                    } else {
                        Poll::Ready(Some(Ok(Frame::data(buf.into()))))
                    }
                }
                Err(e) => Poll::Ready(Some(Err(e))),
            },
            Poll::Pending => Poll::Pending,
        }
    }
}
*/

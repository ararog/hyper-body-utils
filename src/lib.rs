#![doc = include_str!("../README.md")]
#![deny(missing_docs)]
use bytes::Buf;
use futures::ready;

use bytes::Bytes;
use futures::{stream, FutureExt, Stream, TryStreamExt};
use std::pin::Pin;
use std::task::{Context, Poll};

use h3::client::RequestStream as ClientRequestStream;
use h3::server::RequestStream as ServerRequestStream;
use h3_quinn::RecvStream;

pub use http_body_util::BodyExt;

use http_body_util::StreamBody;
use hyper::body::{Body, Frame, Incoming};

use http_body_util::combinators::BoxBody;

#[cfg(test)]
mod tests;

/// Enum to represent different types of HTTP bodies
pub enum HttpBody {
    /// Incoming body from hyper
    Incoming(Incoming),
    /// Boxed stream body
    BoxedStream(BoxBody<Bytes, std::io::Error>),
    /// QUIC client incoming stream
    QuicClientIncoming(ClientRequestStream<RecvStream, Bytes>),
    /// QUIC server incoming stream
    QuicServerIncoming(ServerRequestStream<RecvStream, Bytes>),
}

impl HttpBody {
    /// Create a new HttpBody from an Incoming body
    pub fn from_incoming(incoming: Incoming) -> Self {
        HttpBody::Incoming(incoming)
    }

    /// Create a new HttpBody from a QUIC client stream
    pub fn from_quic_client(stream: ClientRequestStream<RecvStream, Bytes>) -> Self {
        HttpBody::QuicClientIncoming(stream)
    }

    /// Create a new HttpBody from a QUIC server stream
    pub fn from_quic_server(stream: ServerRequestStream<RecvStream, Bytes>) -> Self {
        HttpBody::QuicServerIncoming(stream)
    }

    /// Create a new HttpBody from a text string
    pub fn from_text(text: &str) -> Self {
        Self::from_bytes(text.as_bytes())
    }

    /// Create a new empty HttpBody
    pub fn empty() -> Self {
        Self::from_bytes(&Bytes::new())
    }

    /// Create a new HttpBody from a stream
    pub fn from_stream<S>(stream: S) -> Self
    where
        S: Stream<Item = Result<Frame<Bytes>, std::io::Error>> + Send + Sync + 'static,
    {
        let body = StreamBody::new(stream);
        HttpBody::BoxedStream(BodyExt::boxed(body))
    }

    /// Create a new HttpBody from bytes
    pub fn from_bytes(bytes: &[u8]) -> Self {
        let all_bytes = Bytes::copy_from_slice(bytes);
        let content = stream::iter(vec![Ok(all_bytes)]).map_ok(Frame::data);
        let body = StreamBody::new(content);
        HttpBody::BoxedStream(BodyExt::boxed(body))
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

            HttpBody::BoxedStream(stream) => {
                stream.frame().poll_unpin(cx).map_err(std::io::Error::other)
            }

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

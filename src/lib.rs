#![doc = include_str!("../README.md")]
#![deny(missing_docs)]
#[cfg(feature = "http3")]
use bytes::Buf;
use bytes::Bytes;
#[cfg(all(feature = "http3", feature = "compio"))]
use compio_quic::RecvStream as CompioRecvStream;
#[cfg(feature = "http3")]
use futures::ready;
use futures::{stream, FutureExt, Stream, TryStreamExt};
#[cfg(feature = "http3")]
use h3::{
    client::RequestStream as ClientRequestStream, server::RequestStream as ServerRequestStream,
};
#[cfg(all(feature = "http3", feature = "generic"))]
use h3_quinn::RecvStream as GenericRecvStream;
use http_body_util::{combinators::BoxBody, StreamBody};
use hyper::body::{Body, Frame, Incoming};
use std::{
    pin::Pin,
    task::{Context, Poll},
};

pub use http_body_util::BodyExt;

#[cfg(test)]
mod tests;

/// Enum to represent different types of HTTP bodies
pub enum HttpBody {
    /// Incoming body from hyper
    Incoming(Incoming),
    /// Boxed stream body
    BoxedStream(BoxBody<Bytes, std::io::Error>),
    /// QUIC client incoming stream
    #[cfg(all(feature = "http3", feature = "generic"))]
    GenericClient(ClientRequestStream<GenericRecvStream, Bytes>),
    /// QUIC server incoming stream
    #[cfg(all(feature = "http3", feature = "generic"))]
    GenericServer(ServerRequestStream<GenericRecvStream, Bytes>),
    /// QUIC client incoming stream
    #[cfg(all(feature = "http3", feature = "compio"))]
    CompioClient(ClientRequestStream<CompioRecvStream, Bytes>),
    /// QUIC server incoming stream
    #[cfg(all(feature = "http3", feature = "compio"))]
    CompioServer(ServerRequestStream<CompioRecvStream, Bytes>),
}

impl HttpBody {
    /// Create a new HttpBody from an Incoming body
    pub fn from_incoming(incoming: Incoming) -> Self {
        HttpBody::Incoming(incoming)
    }

    /// Create a new HttpBody from a QUIC client stream
    #[cfg(all(feature = "http3", feature = "generic"))]
    pub fn from_generic_client(stream: ClientRequestStream<GenericRecvStream, Bytes>) -> Self {
        HttpBody::GenericClient(stream)
    }

    /// Create a new HttpBody from a QUIC server stream
    #[cfg(all(feature = "http3", feature = "generic"))]
    pub fn from_generic_server(stream: ServerRequestStream<GenericRecvStream, Bytes>) -> Self {
        HttpBody::GenericServer(stream)
    }

    /// Create a new HttpBody from a QUIC client stream
    #[cfg(all(feature = "http3", feature = "compio"))]
    pub fn from_compio_client(stream: ClientRequestStream<CompioRecvStream, Bytes>) -> Self {
        HttpBody::CompioClient(stream)
    }

    /// Create a new HttpBody from a QUIC server stream
    #[cfg(all(feature = "http3", feature = "compio"))]
    pub fn from_compio_server(stream: ServerRequestStream<CompioRecvStream, Bytes>) -> Self {
        HttpBody::CompioServer(stream)
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

            #[cfg(all(feature = "http3", feature = "generic"))]
            HttpBody::GenericClient(stream) => match ready!(stream.poll_recv_data(cx)) {
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

            #[cfg(all(feature = "http3", feature = "generic"))]
            HttpBody::GenericServer(stream) => match ready!(stream.poll_recv_data(cx)) {
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

            #[cfg(all(feature = "http3", feature = "compio"))]
            HttpBody::CompioClient(stream) => match ready!(stream.poll_recv_data(cx)) {
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

            #[cfg(all(feature = "http3", feature = "compio"))]
            HttpBody::CompioServer(stream) => match ready!(stream.poll_recv_data(cx)) {
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

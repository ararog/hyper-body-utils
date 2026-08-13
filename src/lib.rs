#![doc = include_str!("../README.md")]
#![deny(missing_docs)]
#[cfg(feature = "compio")]
use async_fn_stream::try_fn_stream;
#[cfg(feature = "http3")]
use bytes::Buf;
use bytes::Bytes;
#[cfg(feature = "compio")]
use compio::{fs::File, io::AsyncReadAt, BufResult};
#[cfg(all(feature = "http3", feature = "compio"))]
use compio_quic::RecvStream as CompioRecvStream;
#[cfg(feature = "http3")]
use futures::ready;
#[cfg(feature = "compio")]
use futures::{stream::BoxStream, StreamExt as _};
use futures::{FutureExt, Stream};
#[cfg(feature = "http3")]
use h3::{
    client::RequestStream as ClientRequestStream, server::RequestStream as ServerRequestStream,
};
#[cfg(all(feature = "http3", feature = "generic"))]
use h3_quinn::RecvStream as GenericRecvStream;
use http_body_util::Full;
#[cfg(feature = "generic")]
use http_body_util::{combinators::BoxBody, StreamBody};
use hyper::body::{Body, Frame, Incoming};
#[cfg(feature = "compio")]
use send_wrapper::SendWrapper;
use std::{
    pin::Pin,
    task::{Context, Poll},
};

pub use http_body_util::BodyExt;

#[cfg(test)]
mod tests;

/// Enum to represent different types of HTTP bodies
pub enum HttpBody {
    /// Standard body from hyper holding all request body in memory
    Standard(Full<Bytes>),
    /// Incoming body from hyper, mainly for client responses and server request bodies
    Incoming(Incoming),
    #[cfg(feature = "compio")]
    /// Bytes framed body from compio
    BoxedStream(BoxStream<'static, Result<Bytes, std::io::Error>>),
    #[cfg(feature = "generic")]
    /// Boxed stream body from pool based runtimes
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
    #[cfg(all(feature = "http3", feature = "compio"))]
    CompioServer(ServerRequestStream<CompioRecvStream, Bytes>),
}

impl HttpBody {
    /// Create a new HttpBody from an Incoming body, you can use this method
    /// to hold client response body or server incoming request body.
    ///
    /// # Arguments
    /// * `incoming` - The Incoming body to create the HttpBody from
    ///
    /// # Returns
    /// * `HttpBody` - The created HttpBody
    pub fn from_incoming(incoming: Incoming) -> Self {
        HttpBody::Incoming(incoming)
    }

    /// Create a new HttpBody from a QUIC client stream, it is intended to be used
    /// with pool based runtimes only.
    ///
    /// # Arguments
    /// * `stream` - The QUIC client stream to create the HttpBody from
    ///
    /// # Returns
    /// * `HttpBody` - The created HttpBody
    #[cfg(all(feature = "http3", feature = "generic"))]
    pub fn from_generic_client(stream: ClientRequestStream<GenericRecvStream, Bytes>) -> Self {
        HttpBody::GenericClient(stream)
    }

    /// Create a new HttpBody from a QUIC server stream, it is intended to be used
    /// with pool based runtimes only.
    ///
    /// # Arguments
    /// * `stream` - The QUIC server stream to create the HttpBody from
    ///
    /// # Returns
    /// * `HttpBody` - The created HttpBody
    #[cfg(all(feature = "http3", feature = "generic"))]
    pub fn from_generic_server(stream: ServerRequestStream<GenericRecvStream, Bytes>) -> Self {
        HttpBody::GenericServer(stream)
    }

    /// Create a new HttpBody from a QUIC client stream, it is intended to be used
    /// with Compio runtime only.
    ///
    /// # Arguments
    /// * `stream` - The QUIC client stream to create the HttpBody from
    ///
    /// # Returns
    /// * `HttpBody` - The created HttpBody
    #[cfg(all(feature = "http3", feature = "compio"))]
    pub fn from_compio_client(stream: ClientRequestStream<CompioRecvStream, Bytes>) -> Self {
        HttpBody::CompioClient(stream)
    }

    /// Create a new HttpBody from a QUIC server stream, it is intended to be used
    /// with Compio runtime only.
    ///
    /// # Arguments
    /// * `stream` - The QUIC server stream to create the HttpBody from
    ///
    /// # Returns
    /// * `HttpBody` - The created HttpBody
    #[cfg(all(feature = "http3", feature = "compio"))]
    pub fn from_compio_server(stream: ServerRequestStream<CompioRecvStream, Bytes>) -> Self {
        HttpBody::CompioServer(stream)
    }

    /// Create a new HttpBody from a text string
    ///
    /// # Arguments
    /// * `text` - The text to create the HttpBody from
    ///
    /// # Returns
    /// * `HttpBody` - The created HttpBody
    ///
    /// # Notes
    /// * This method is not intended for large amounts of data
    pub fn from_text(text: &str) -> Self {
        Self::from_bytes(text.as_bytes())
    }

    /// Create a new HttpBody from bytes
    ///
    /// # Arguments
    /// * `bytes` - The bytes to create the HttpBody from
    ///
    /// # Returns
    /// * `HttpBody` - The created HttpBody
    ///
    /// # Notes
    /// * This method is not intended for large amounts of data
    pub fn from_bytes(bytes: &[u8]) -> Self {
        let all_bytes = Bytes::copy_from_slice(bytes);
        HttpBody::Standard(Full::new(all_bytes))
    }

    #[cfg(feature = "generic")]
    /// Create a new HttpBody from a stream
    ///
    /// # Arguments
    /// * `stream` - The stream to create the HttpBody from
    ///
    /// # Returns
    /// * `HttpBody` - The created HttpBody
    ///
    /// # Notes
    /// * This method is intended for use with streams that are already boxed
    /// * You can't clone the stream, so you can't use it multiple times
    pub fn from_stream<S>(stream: S) -> Self
    where
        S: Stream<Item = Result<Frame<Bytes>, std::io::Error>> + Send + Sync + 'static,
    {
        let body = StreamBody::new(stream);
        HttpBody::BoxedStream(BodyExt::boxed(body))
    }

    #[cfg(feature = "compio")]
    /// Create a new HttpBody from a stream
    ///
    /// # Arguments
    /// * `stream` - The stream to create the HttpBody from
    ///
    /// # Returns
    /// * `HttpBody` - The created HttpBody
    ///
    /// # Notes
    /// * This method is intended for use with streams that are already boxed
    /// * You can't clone the stream, so you can't use it multiple times
    pub fn from_stream<S>(stream: S) -> Self
    where
        S: Stream<Item = Result<Bytes, std::io::Error>> + Send + 'static,
    {
        HttpBody::BoxedStream(stream.boxed())
    }

    /// Check if the HttpBody is a stream
    ///
    /// # Returns
    /// * `bool` - True if the HttpBody is a stream, false otherwise
    pub fn is_stream(&self) -> bool {
        #[cfg(feature = "generic")]
        {
            matches!(self, HttpBody::BoxedStream(_))
        }
        #[cfg(feature = "compio")]
        {
            matches!(self, HttpBody::BoxedStream(_))
        }
    }

    /// Create a new empty HttpBody
    ///
    /// # Returns
    /// * `HttpBody` - The created empty HttpBody
    pub fn empty() -> Self {
        Self::from_bytes(&Bytes::new())
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
            HttpBody::Standard(full_body) => full_body
                .frame()
                .poll_unpin(cx)
                .map_err(std::io::Error::other),

            HttpBody::Incoming(incoming) => incoming
                .frame()
                .poll_unpin(cx)
                .map_err(std::io::Error::other),

            #[cfg(feature = "compio")]
            HttpBody::BoxedStream(stream) => stream
                .poll_next_unpin(cx)
                .map(|b| b.map(|b| b.map(Frame::data))),

            #[cfg(feature = "generic")]
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

impl From<&str> for HttpBody {
    fn from(value: &str) -> Self {
        HttpBody::from_text(value)
    }
}

impl From<String> for HttpBody {
    fn from(value: String) -> Self {
        HttpBody::from_text(&value)
    }
}

impl From<&[u8]> for HttpBody {
    fn from(value: &[u8]) -> Self {
        HttpBody::from_bytes(value)
    }
}

impl From<Vec<u8>> for HttpBody {
    fn from(value: Vec<u8>) -> Self {
        HttpBody::from_bytes(&value)
    }
}

impl From<Bytes> for HttpBody {
    fn from(value: Bytes) -> Self {
        HttpBody::from_bytes(&value)
    }
}

#[cfg(feature = "compio")]
impl From<File> for HttpBody {
    fn from(value: File) -> Self {
        let stream = from_asyncread(value);
        HttpBody::from_stream(SendWrapper::new(stream))
    }
}

#[cfg(feature = "compio")]
fn from_asyncread<R>(reader: R) -> impl Stream<Item = Result<Bytes, std::io::Error>>
where
    R: AsyncReadAt,
{
    try_fn_stream(|emitter| async move {
        let mut pos = 0;
        loop {
            let buf = Vec::with_capacity(4096);
            let BufResult(res, buffer) = reader.read_at(buf, pos).await;
            let len = res?;
            if len == 0 {
                break Ok(());
            }
            pos += len as u64;
            emitter.emit(Bytes::from(buffer)).await
        }
    })
}

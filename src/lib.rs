#![doc = include_str!("../README.md")]
#![deny(missing_docs)]
#[cfg(any(feature = "compio-h3", feature = "generic-h3"))]
use bytes::Buf as _;
use bytes::Bytes;
#[cfg(any(feature = "compio-h3", feature = "generic-h3"))]
use futures::ready;
#[cfg(feature = "compio")]
use futures::StreamExt as _;
use futures::{FutureExt, Stream};
use http_body_util::Full;
use hyper::body::{Body, Frame, Incoming};
use std::{
    io::Error,
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
    #[cfg(feature = "generic")]
    /// Boxed stream body from pool based runtimes
    GenericStream(http_body_util::combinators::BoxBody<Bytes, std::io::Error>),
    #[cfg(feature = "compio")]
    /// Bytes framed body from compio
    CompioStream(futures::stream::BoxStream<'static, Result<Bytes, std::io::Error>>),
    /// QUIC client incoming stream
    #[cfg(feature = "generic-h3")]
    GenericClient(h3::client::RequestStream<h3_quinn::RecvStream, Bytes>),
    /// QUIC server incoming stream
    #[cfg(feature = "generic-h3")]
    GenericServer(h3::server::RequestStream<h3_quinn::RecvStream, Bytes>),
    /// QUIC client incoming stream
    #[cfg(feature = "compio-h3")]
    CompioClient(compio_quic::h3::client::RequestStream<compio_quic::RecvStream, Bytes>),
    /// QUIC server incoming stream
    #[cfg(feature = "compio-h3")]
    CompioServer(compio_quic::h3::server::RequestStream<compio_quic::RecvStream, Bytes>),
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
    #[cfg(feature = "generic-h3")]
    pub fn from_generic_client(
        stream: h3::client::RequestStream<h3_quinn::RecvStream, Bytes>,
    ) -> Self {
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
    #[cfg(feature = "generic-h3")]
    pub fn from_generic_server(
        stream: h3::server::RequestStream<h3_quinn::RecvStream, Bytes>,
    ) -> Self {
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
    #[cfg(feature = "compio-h3")]
    pub fn from_compio_client(
        stream: compio_quic::h3::client::RequestStream<compio_quic::RecvStream, Bytes>,
    ) -> Self {
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
    #[cfg(feature = "compio-h3")]
    pub fn from_compio_server(
        stream: compio_quic::h3::server::RequestStream<compio_quic::RecvStream, Bytes>,
    ) -> Self {
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
    pub fn from_generic_stream<S>(stream: S) -> Self
    where
        S: Stream<Item = Result<Frame<Bytes>, Error>> + Send + Sync + 'static,
    {
        let body = http_body_util::StreamBody::new(stream);
        HttpBody::GenericStream(body.boxed())
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
    pub fn from_compio_stream<S>(stream: S) -> Self
    where
        S: Stream<Item = Result<Bytes, Error>> + Send + 'static,
    {
        HttpBody::CompioStream(stream.boxed())
    }

    /// Create a new empty HttpBody
    ///
    /// # Returns
    /// * `HttpBody` - The created empty HttpBody
    pub fn empty() -> Self {
        Self::from_bytes(&Bytes::new())
    }

    /// Try to clone the HttpBody, if it is a stream, it will return None
    ///
    /// # Returns
    /// * `Option<HttpBody>` - Some(HttpBody) if it can be cloned,
    pub fn try_clone(&self) -> Result<Self, Error> {
        match self {
            HttpBody::Standard(content) => Ok(HttpBody::Standard(content.clone())),
            _ => Err(Error::new(
                std::io::ErrorKind::Other,
                "Cannot clone stream body",
            )),
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
            HttpBody::Standard(full_body) => full_body.frame().poll_unpin(cx).map_err(Error::other),

            HttpBody::Incoming(incoming) => incoming.frame().poll_unpin(cx).map_err(Error::other),

            #[cfg(feature = "generic")]
            HttpBody::GenericStream(stream) => stream.frame().poll_unpin(cx).map_err(Error::other),

            #[cfg(feature = "compio")]
            HttpBody::CompioStream(stream) => stream
                .poll_next_unpin(cx)
                .map(|b| b.map(|b| b.map(Frame::data))),

            #[cfg(feature = "generic-h3")]
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
                Err(e) => Poll::Ready(Some(Err(Error::other(e)))),
            },

            #[cfg(feature = "generic-h3")]
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
                Err(e) => Poll::Ready(Some(Err(Error::other(e)))),
            },

            #[cfg(feature = "compio-h3")]
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
                Err(e) => Poll::Ready(Some(Err(Error::other(e)))),
            },

            #[cfg(feature = "compio-h3")]
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
                Err(e) => Poll::Ready(Some(Err(Error::other(e)))),
            },
        }
    }
}

impl Stream for HttpBody {
    type Item = Result<Frame<Bytes>, Error>;

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
impl From<compio::fs::File> for HttpBody {
    fn from(value: compio::fs::File) -> Self {
        let stream = from_asyncread(value);
        HttpBody::from_compio_stream(send_wrapper::SendWrapper::new(stream))
    }
}

#[cfg(feature = "compio")]
fn from_asyncread<R>(reader: R) -> impl Stream<Item = Result<Bytes, Error>>
where
    R: compio::io::AsyncReadAt,
{
    async_fn_stream::try_fn_stream(|emitter| async move {
        let mut pos = 0;
        loop {
            let buf = Vec::with_capacity(4096);
            let compio::BufResult(res, buffer) = reader.read_at(buf, pos).await;
            let len = res?;
            if len == 0 {
                break Ok(());
            }
            pos += len as u64;
            emitter.emit(Bytes::from(buffer)).await
        }
    })
}

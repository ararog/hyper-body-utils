#![doc = include_str!("../README.md")]
#![deny(missing_docs)]
#[cfg(feature = "h3")]
use bytes::Buf as _;
use bytes::Bytes;
#[cfg(feature = "h3")]
use futures::ready;
use futures::{FutureExt, Stream};
#[cfg(feature = "h3")]
use h3_quinn::{BidiStream, RecvStream};
use http_body_util::{combinators::BoxBody, BodyExt, Full};
use hyper::body::{Body, Frame, Incoming};
use std::{
    fmt::{Debug, Formatter},
    io::Error,
    pin::Pin,
    task::{Context, Poll},
};

#[cfg(test)]
mod tests;

/// Enum to represent different types of HTTP bodies
pub enum HttpBody {
    /// Standard body from hyper holding all request body in memory
    Standard(Full<Bytes>),
    /// Incoming body from hyper, mainly for client responses and server request bodies
    Incoming(Incoming),
    /// Boxed stream body from pool based runtimes
    Stream(BoxBody<Bytes, std::io::Error>),
    #[cfg(feature = "h3")]
    /// Enum for quic related bodies    
    Quic(QuicBody),
}

#[cfg(feature = "h3")]
/// QuicBody variantes for client and server
pub enum QuicBody {
    /// QUIC client incoming stream
    Client(ClientBody),
    /// QUIC server incoming stream
    Server(ServerBody),
}

#[cfg(feature = "h3")]
/// ClientBody variantes for recv and bidi
pub enum ClientBody {
    /// QUIC client recv stream
    Recv(h3::client::RequestStream<RecvStream, Bytes>),
    /// QUIC client bidy stream
    Bidi(h3::client::RequestStream<BidiStream<Bytes>, Bytes>),
}

#[cfg(feature = "h3")]
/// ServerBody variantes for recv and bidi
pub enum ServerBody {
    /// QUIC client recv stream
    Recv(h3::server::RequestStream<RecvStream, Bytes>),
    /// QUIC server bidi stream
    Bidi(h3::server::RequestStream<BidiStream<Bytes>, Bytes>),
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
    pub fn incoming(incoming: Incoming) -> Self {
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
    #[cfg(feature = "h3")]
    pub fn quic_client_recv(
        stream: h3::client::RequestStream<h3_quinn::RecvStream, Bytes>,
    ) -> Self {
        HttpBody::Quic(QuicBody::Client(ClientBody::Recv(stream)))
    }

    /// Create a new HttpBody from a QUIC server stream, it is intended to be used
    /// with pool based runtimes only.
    ///
    /// # Arguments
    /// * `stream` - The QUIC server stream to create the HttpBody from
    ///
    /// # Returns
    /// * `HttpBody` - The created HttpBody
    #[cfg(feature = "h3")]
    pub fn server_recv_stream(
        stream: h3::server::RequestStream<h3_quinn::RecvStream, Bytes>,
    ) -> Self {
        HttpBody::Quic(QuicBody::Server(ServerBody::Recv(stream)))
    }

    /// Create a new HttpBody from a QUIC client stream, it is intended to be used
    /// with pool based runtimes only.
    ///
    /// # Arguments
    /// * `stream` - The QUIC client stream to create the HttpBody from
    ///
    /// # Returns
    /// * `HttpBody` - The created HttpBody
    #[cfg(feature = "h3")]
    pub fn client_bidi_stream(
        stream: h3::client::RequestStream<h3_quinn::BidiStream<Bytes>, Bytes>,
    ) -> Self {
        HttpBody::Quic(QuicBody::Client(ClientBody::Bidi(stream)))
    }

    /// Create a new HttpBody from a QUIC server stream, it is intended to be used
    /// with pool based runtimes only.
    ///
    /// # Arguments
    /// * `stream` - The QUIC server stream to create the HttpBody from
    ///
    /// # Returns
    /// * `HttpBody` - The created HttpBody
    #[cfg(feature = "h3")]
    pub fn server_bidi_stream(
        stream: h3::server::RequestStream<h3_quinn::BidiStream<Bytes>, Bytes>,
    ) -> Self {
        HttpBody::Quic(QuicBody::Server(ServerBody::Bidi(stream)))
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
    pub fn text(text: &str) -> Self {
        Self::bytes(text.as_bytes())
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
    pub fn bytes(bytes: &[u8]) -> Self {
        let all_bytes = Bytes::copy_from_slice(bytes);
        HttpBody::Standard(Full::new(all_bytes))
    }

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
    pub fn stream<S>(stream: S) -> Self
    where
        S: Stream<Item = Result<Frame<Bytes>, Error>> + Send + Sync + 'static,
    {
        let body = http_body_util::StreamBody::new(stream);
        HttpBody::Stream(http_body_util::BodyExt::boxed(body))
    }

    /// Create a new empty HttpBody
    ///
    /// # Returns
    /// * `HttpBody` - The created empty HttpBody
    pub fn empty() -> Self {
        Self::bytes(&Bytes::new())
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

impl Default for HttpBody {
    fn default() -> Self {
        HttpBody::empty()
    }
}

impl Debug for HttpBody {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            HttpBody::Standard(content) => {
                if let Some(content) = content.clone().into_inner() {
                    f.debug_tuple("HttpBody::Standard").field(&content).finish()
                } else {
                    f.debug_tuple("HttpBody::Standard").finish()
                }
            }
            HttpBody::Incoming(_) => f.debug_tuple("HttpBody::Incoming").finish(),
            HttpBody::Stream(_) => f.debug_tuple("HttpBody::Stream").finish(),
            #[cfg(feature = "h3")]
            HttpBody::Quic(_) => f.debug_tuple("HttpBody::Quic").finish(),
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

            HttpBody::Stream(stream) => stream.frame().poll_unpin(cx).map_err(Error::other),

            #[cfg(feature = "h3")]
            HttpBody::Quic(stream) => match stream {
                QuicBody::Client(inner) => match inner {
                    ClientBody::Bidi(stream) => match ready!(stream.poll_recv_data(cx)) {
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
                    ClientBody::Recv(stream) => match ready!(stream.poll_recv_data(cx)) {
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
                },

                QuicBody::Server(inner) => match inner {
                    ServerBody::Bidi(stream) => match ready!(stream.poll_recv_data(cx)) {
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
                    ServerBody::Recv(stream) => match ready!(stream.poll_recv_data(cx)) {
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
                },
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
        HttpBody::text(value)
    }
}

impl From<String> for HttpBody {
    fn from(value: String) -> Self {
        HttpBody::text(&value)
    }
}

impl From<&[u8]> for HttpBody {
    fn from(value: &[u8]) -> Self {
        HttpBody::bytes(value)
    }
}

impl From<Vec<u8>> for HttpBody {
    fn from(value: Vec<u8>) -> Self {
        HttpBody::bytes(&value)
    }
}

impl From<Bytes> for HttpBody {
    fn from(value: Bytes) -> Self {
        HttpBody::bytes(&value)
    }
}

#[cfg(feature = "h3")]
impl From<h3::client::RequestStream<RecvStream, Bytes>> for HttpBody {
    fn from(value: h3::client::RequestStream<RecvStream, Bytes>) -> Self {
        HttpBody::Quic(QuicBody::Client(ClientBody::Recv(value)))
    }
}

#[cfg(feature = "h3")]
impl From<h3::client::RequestStream<BidiStream<Bytes>, Bytes>> for HttpBody {
    fn from(value: h3::client::RequestStream<BidiStream<Bytes>, Bytes>) -> Self {
        HttpBody::Quic(QuicBody::Client(ClientBody::Bidi(value)))
    }
}

#[cfg(feature = "h3")]
impl From<h3::server::RequestStream<RecvStream, Bytes>> for HttpBody {
    fn from(value: h3::server::RequestStream<RecvStream, Bytes>) -> Self {
        HttpBody::Quic(QuicBody::Server(ServerBody::Recv(value)))
    }
}

#[cfg(feature = "h3")]
impl From<h3::server::RequestStream<BidiStream<Bytes>, Bytes>> for HttpBody {
    fn from(value: h3::server::RequestStream<BidiStream<Bytes>, Bytes>) -> Self {
        HttpBody::Quic(QuicBody::Server(ServerBody::Bidi(value)))
    }
}

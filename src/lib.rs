#[cfg(feature = "http3")]
use bytes::Buf;
#[cfg(feature = "http3")]
use futures::ready;

use bytes::Bytes;
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

use http_body_util::StreamBody;
use hyper::body::{Body, Frame, Incoming};

use http_body_util::combinators::BoxBody;

// #[cfg(feature = "compio-rt")]
// use send_wrapper::SendWrapper;

mod tests;

pub enum HttpBody {
    Incoming(Incoming),
    BoxedStream(BoxBody<Bytes, std::io::Error>),
    // #[cfg(feature = "compio-rt")]
    // WrappedStream(Pin<Box<dyn Stream<Item = Result<Frame<Bytes>, std::io::Error>> + Send>>),
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

    #[cfg(any(feature = "tokio-rt", feature = "smol-rt"))]
    pub fn from_text(text: &str) -> Self {
        Self::from_bytes(text.as_bytes())
    }

    // #[cfg(feature = "compio-rt")]
    // pub fn wrap_text(text: &str) -> Self {
    //     Self::wrap_bytes(text.as_bytes())
    // }

    #[cfg(any(feature = "tokio-rt", feature = "smol-rt"))]
    pub fn from_stream<S>(stream: S) -> Self
    where
        S: Stream<Item = Result<Frame<Bytes>, std::io::Error>> + Send + Sync + 'static,
    {
        let body = StreamBody::new(stream);
        HttpBody::BoxedStream(BodyExt::boxed(body))
    }

    // #[cfg(feature = "compio-rt")]
    // pub fn from_cursor(cursor: Cursor<_>) -> Self {
    //     let content = cursor.read_only().bytes();
    //     let body = StreamBody::new(content.map_ok(Frame::data));
    //     HttpBody::WrappedStream(Box::pin(SendWrapper::new(body)))
    // }

    pub fn from_bytes(bytes: &[u8]) -> Self {
        let all_bytes = Bytes::copy_from_slice(bytes);
        let content = stream::iter(vec![Ok(all_bytes)]).map_ok(Frame::data);
        let body = StreamBody::new(content);
        HttpBody::BoxedStream(BodyExt::boxed(body))
    }

    // #[cfg(feature = "compio-rt")]
    // pub fn wrap_bytes(bytes: &[u8]) -> Self {
    //     let all_bytes = Bytes::copy_from_slice(bytes);
    //     let content = stream::iter(vec![Ok(all_bytes)]).map_ok(Frame::data);
    //     let body = StreamBody::new(content);
    //     HttpBody::WrappedStream(Box::pin(SendWrapper::new(body)))
    // }
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

            // #[cfg(feature = "compio-rt")]
            // HttpBody::WrappedStream(stream) => {
            //     stream::StreamExt::poll_next_unpin(stream, cx).map_err(std::io::Error::other)
            // }
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

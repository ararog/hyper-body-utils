#[cfg(feature = "http3")]
use bytes::Buf;

use bytes::Bytes;
use futures::{stream, FutureExt, Stream, TryStreamExt};

#[cfg(feature = "http3")]
use h3::client::RequestStream as ClientRequestStream;
#[cfg(feature = "http3")]
use h3::server::RequestStream as ServerRequestStream;
#[cfg(feature = "http3")]
use h3_quinn::{SendStream, RecvStream};

use http_body_util::{combinators::BoxBody, BodyExt, StreamBody};
use hyper::body::{Body, Frame, Incoming};

#[cfg(feature = "smol-rt")]
use smol::fs::File;
#[cfg(feature = "smol-rt")]
use smol::io::AsyncReadExt;

#[cfg(feature = "tokio-rt")]
use tokio::fs::File;
#[cfg(feature = "tokio-rt")]
use tokio_util::io::ReaderStream;

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
            let content = ReaderStream::new(file).map_ok(Frame::data);
            let body = StreamBody::new(content);
            HttpBody::Stream(body.boxed())
        }

        #[cfg(feature = "smol-rt")]
        {
            // TODO: This is not right, I'm mapping all slices and placing them in memory
            let content = file
                .bytes()
                .map_ok(|data| Frame::data(bytes::Bytes::copy_from_slice(&[data])));
            let body = StreamBody::new(content);
            HttpBody::Stream(body.boxed())
        }
    }

    pub fn from_bytes(bytes: &[u8]) -> Self {
        #[cfg(feature = "tokio-rt")]
        {
            let all_bytes = Bytes::copy_from_slice(bytes);
            let content = stream::iter(vec![Ok(all_bytes)]).map_ok(Frame::data);
            let body = StreamBody::new(content);
            HttpBody::Stream(body.boxed())
        }

        #[cfg(feature = "smol-rt")]
        {
            let all_bytes = Bytes::copy_from_slice(bytes);
            let content = stream::iter(vec![Ok(all_bytes)]).map_ok(Frame::data);
            let body = StreamBody::new(content);
            HttpBody::Stream(body.boxed())
        }
    }
}

impl Body for HttpBody {
    type Data = Bytes;

    type Error = std::io::Error;

    fn poll_frame(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Result<Frame<Self::Data>, Self::Error>>> {
        match self.get_mut() {
            HttpBody::Incoming(incoming) => incoming
                .frame()
                .poll_unpin(cx)
                .map_err(std::io::Error::other),
            HttpBody::Stream(stream) => {
                stream.frame().poll_unpin(cx).map_err(std::io::Error::other)
            }
            #[cfg(feature = "http3")]
            HttpBody::QuicClientIncoming(stream) => stream.poll_recv_data(cx).map(|value| match value {
                Ok(Some(mut value)) => {
                    Some(Ok(Frame::data(value.copy_to_bytes(value.remaining()))))
                }
                Ok(None) => None,
                Err(e) => Some(Err(std::io::Error::other(e))),
            }),
            #[cfg(feature = "http3")]
            HttpBody::QuicServerIncoming(stream) => stream.poll_recv_data(cx).map(|value| match value {
                Ok(Some(mut value)) => {
                    Some(Ok(Frame::data(value.copy_to_bytes(value.remaining()))))
                }
                Ok(None) => None,
                Err(e) => Some(Err(std::io::Error::other(e))),
            }),
        }
    }
}

impl Stream for HttpBody {
    type Item = Result<Frame<Bytes>, std::io::Error>;

    fn poll_next(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        self.poll_frame(cx)
    }
}

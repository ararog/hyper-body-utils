use bytes::Bytes;
use futures::{FutureExt, Stream, TryStreamExt, stream};
use http_body_util::{BodyExt, StreamBody, combinators::BoxBody};
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
}

impl HttpBody {
    pub fn from_incoming(incoming: Incoming) -> Self {
        HttpBody::Incoming(incoming)
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

use crate::HttpBody;
use bytes::Bytes;
use futures::{StreamExt, TryStreamExt};
use hyper::body::Frame;
use macro_rules_attribute::apply;
use smol_macros::test;

#[tokio::test]
async fn test_file_tokio() -> Result<(), std::io::Error> {
    use tokio::fs::File;
    let file = File::open("src/tests/files/index.html").await?;
    let content = tokio_util::io::ReaderStream::new(file).map_ok(Frame::data);
    let mut body = HttpBody::from_stream(content);
    let mut buffer = Vec::new();
    while let Some(Ok(chunk)) = body.next().await {
        if let Ok(chunk) = chunk.into_data() {
            buffer.extend_from_slice(&chunk);
        }
    }
    assert_eq!(buffer, b"<html>\n<head>\n  <title>\n    Tested!\n  </title>\n</head>\n<body>\n  <p>\n    Tested!\n  </p>\n</body>\n</html>");
    Ok(())
}

#[tokio::test]
async fn test_bytes_tokio() -> Result<(), std::io::Error> {
    let content = Bytes::from_static(b"<html>\n<head>\n  <title>\n    Tested!\n  </title>\n</head>\n<body>\n  <p>\n    Tested!\n  </p>\n</body>\n</html>");
    let mut body = HttpBody::from_bytes(&content);
    let mut buffer = Vec::new();
    while let Some(Ok(chunk)) = body.next().await {
        if let Ok(chunk) = chunk.into_data() {
            buffer.extend_from_slice(&chunk);
        }
    }
    assert_eq!(buffer, b"<html>\n<head>\n  <title>\n    Tested!\n  </title>\n</head>\n<body>\n  <p>\n    Tested!\n  </p>\n</body>\n</html>");
    Ok(())
}

#[apply(test!)]
async fn test_file_smol() -> Result<(), std::io::Error> {
    use smol::fs::File;
    use smol::io::AsyncReadExt;
    let file = File::open("src/tests/files/index.html").await?;
    let content = file
        .bytes()
        .map_ok(|data| Frame::data(Bytes::copy_from_slice(&[data])));
    let mut body = HttpBody::from_stream(content);
    let mut buffer = Vec::new();
    while let Some(Ok(chunk)) = body.next().await {
        if let Ok(chunk) = chunk.into_data() {
            buffer.extend_from_slice(&chunk);
        }
    }
    assert_eq!(buffer, b"<html>\n<head>\n  <title>\n    Tested!\n  </title>\n</head>\n<body>\n  <p>\n    Tested!\n  </p>\n</body>\n</html>");
    Ok(())
}

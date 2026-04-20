use futures::{StreamExt, TryStreamExt};
use hyper::body::Frame;

#[cfg(feature = "smol-rt")]
use macro_rules_attribute::apply;
#[cfg(feature = "smol-rt")]
use smol_macros::test;

use crate::HttpBody;

#[cfg(feature = "tokio-rt")]
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

#[cfg(feature = "smol-rt")]
#[apply(test!)]
async fn test_file_smol() -> Result<(), std::io::Error> {
    use smol::fs::File;
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

// #[cfg(feature = "compio-rt")]
// #[compio::test]
// async fn test_file_compio() -> Result<(), std::io::Error> {
//     use compio_fs::File;
//     do_test_file().await
// }

use std::io::Cursor;

use crate::HttpBody;
use compio::{fs::File, io::AsyncReadExt};
use futures::StreamExt;
use send_wrapper::SendWrapper;

#[compio::test]
async fn test_file_compio() -> Result<(), std::io::Error> {
    let file = File::open("src/tests/files/index.html").await?;
    let mut body = HttpBody::from(file);
    let mut buffer = Vec::new();
    while let Some(Ok(chunk)) = body.next().await {
        if let Ok(chunk) = chunk.into_data() {
            buffer.extend_from_slice(&chunk);
        }
    }
    assert_eq!(buffer, b"<html>\n<head>\n  <title>\n    Tested!\n  </title>\n</head>\n<body>\n  <p>\n    Tested!\n  </p>\n</body>\n</html>");
    Ok(())
}

#[compio::test]
async fn test_stream_compio() -> Result<(), std::io::Error> {
    let file = File::open("src/tests/files/index.html").await?;
    let mut stream = Cursor::new(file).read_only().bytes();
    let mut body = HttpBody::from_compio_stream(SendWrapper::new(stream));
    let mut buffer = Vec::new();
    while let Some(Ok(chunk)) = body.next().await {
        if let Ok(chunk) = chunk.into_data() {
            buffer.extend_from_slice(&chunk);
        }
    }
    assert_eq!(buffer, b"<html>\n<head>\n  <title>\n    Tested!\n  </title>\n</head>\n<body>\n  <p>\n    Tested!\n  </p>\n</body>\n</html>");
    Ok(())
}

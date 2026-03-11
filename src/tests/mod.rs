use futures::StreamExt;
#[cfg(feature = "smol-rt")]
use macro_rules_attribute::apply;
#[cfg(feature = "smol-rt")]
use smol_macros::test;

#[cfg(feature = "compio-rt")]
use compio_fs::File;
#[cfg(feature = "smol-rt")]
use smol::fs::File;
#[cfg(feature = "tokio-rt")]
use tokio::fs::File;

use crate::HttpBody;

pub async fn do_test_file() -> Result<(), std::io::Error> {
    let file = File::open("src/tests/files/index.html").await?;
    let mut body = HttpBody::from_file(file);
    let mut buffer = Vec::new();
    while let Some(Ok(chunk)) = body.next().await {
        if let Ok(chunk) = chunk.into_data() {
            buffer.extend_from_slice(&chunk);
        }
    }
    assert_eq!(buffer, b"<html>\n<head>\n  <title>\n    Tested!\n  </title>\n</head>\n<body>\n  <p>\n    Tested!\n  </p>\n</body>\n</html>");
    Ok(())
}

#[cfg(feature = "tokio-rt")]
#[tokio::test]
async fn test_file() -> Result<(), std::io::Error> {
    do_test_file().await
}

#[cfg(feature = "smol-rt")]
#[apply(test!)]
async fn test_file() -> Result<(), std::io::Error> {
    do_test_file().await
}

#[cfg(feature = "compio-rt")]
#[compio::test]
async fn test_file() -> Result<(), std::io::Error> {
    do_test_file().await
}

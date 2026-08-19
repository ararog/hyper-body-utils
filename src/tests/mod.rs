use crate::HttpBody;
use bytes::Bytes;
use hyper::body::Body as _;
use std::io::Error;

#[cfg(feature = "compio")]
mod compio;
#[cfg(feature = "generic")]
mod generic;

#[test]
fn test_empty() -> Result<(), Error> {
    let body = HttpBody::empty();
    assert!(matches!(body, HttpBody::Standard(empty) if empty.is_end_stream()));
    Ok(())
}

#[test]
fn test_clone() -> Result<(), Error> {
    let body_a = HttpBody::from_bytes(b"test");

    let Ok(HttpBody::Standard(body_b)) = body_a.try_clone() else {
        panic!("Expected Standard body")
    };

    match body_a {
        HttpBody::Standard(bytes) => assert_eq!(bytes.into_inner(), body_b.into_inner()),
        _ => panic!("Should not happen"),
    };

    Ok(())
}

#[test]
fn test_from_bytes() -> Result<(), Error> {
    let body = HttpBody::from_bytes(b"test");
    match body {
        HttpBody::Standard(e) => {
            assert_eq!(e.into_inner(), Some(Bytes::from_static(b"test")));
        }
        _ => panic!("Expected Standard body"),
    }
    Ok(())
}

#[test]
fn test_from_string() -> Result<(), Error> {
    let body = HttpBody::from("some heap string".to_string());
    match body {
        HttpBody::Standard(e) => {
            assert_eq!(
                e.into_inner(),
                Some(Bytes::from_static(b"some heap string"))
            );
        }
        _ => panic!("Expected Standard body"),
    }
    Ok(())
}

#[test]
fn test_from_str() -> Result<(), Error> {
    let body = HttpBody::from("some text");
    match body {
        HttpBody::Standard(e) => {
            assert_eq!(e.into_inner(), Some(Bytes::from_static(b"some text")));
        }
        _ => panic!("Expected Standard body"),
    }
    Ok(())
}

#[test]
fn test_from_vec() -> Result<(), Error> {
    let body = HttpBody::from(b"some text".to_vec());
    match body {
        HttpBody::Standard(e) => {
            assert_eq!(e.into_inner(), Some(Bytes::from_static(b"some text")));
        }
        _ => panic!("Expected Standard body"),
    }
    Ok(())
}

#[test]
fn test_from_slice() -> Result<(), Error> {
    let body = HttpBody::from(b"some text".as_slice());
    match body {
        HttpBody::Standard(e) => {
            assert_eq!(e.into_inner(), Some(Bytes::from_static(b"some text")));
        }
        _ => panic!("Expected Standard body"),
    }
    Ok(())
}

#[test]
fn test_from_bytes_bytes() -> Result<(), Error> {
    let body = HttpBody::from(Bytes::from_static(b"some text"));
    match body {
        HttpBody::Standard(e) => {
            assert_eq!(e.into_inner(), Some(Bytes::from_static(b"some text")));
        }
        _ => panic!("Expected Standard body"),
    }
    Ok(())
}

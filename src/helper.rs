//! Helper functions allowing you to avoid writing boilerplate code for common operations, such as
//! parsing JSON or reading files.

// Copyright (c) 2016 Google Inc (lewinb@google.com).
//
// Refer to the project root for licensing information.
use crate::service_account::ServiceAccountKey;
use crate::types::{ApplicationSecret, ConsoleApplicationSecret};

use std::io;
use std::path::Path;
use std::future::Future;

use futures::future::FutureExt;
use tokio::sync::oneshot;

/// Read an application secret from a file.
pub async fn read_application_secret<P: AsRef<Path>>(path: P) -> io::Result<ApplicationSecret> {
    parse_application_secret(tokio::fs::read(path).await?)
}

/// Read an application secret from a JSON string.
pub fn parse_application_secret<S: AsRef<[u8]>>(secret: S) -> io::Result<ApplicationSecret> {
    let decoded: ConsoleApplicationSecret =
        serde_json::from_slice(secret.as_ref()).map_err(|e| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Bad application secret: {}", e),
            )
        })?;

    if let Some(web) = decoded.web {
        Ok(web)
    } else if let Some(installed) = decoded.installed {
        Ok(installed)
    } else {
        Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "Unknown application secret format",
        ))
    }
}

/// Read a service account key from a JSON file. You can download the JSON keys from the Google
/// Cloud Console or the respective console of your service provider.
pub async fn read_service_account_key<P: AsRef<Path>>(path: P) -> io::Result<ServiceAccountKey> {
    let key = tokio::fs::read(path).await?;
    serde_json::from_slice(&key).map_err(|e| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("Bad service account key: {}", e),
        )
    })
}

pub(crate) fn join<T>(pieces: &[T], separator: &str) -> String
where
    T: AsRef<str>,
{
    let mut iter = pieces.iter();
    let first = match iter.next() {
        Some(p) => p,
        None => return String::new(),
    };
    let num_separators = pieces.len() - 1;
    let pieces_size: usize = pieces.iter().map(|p| p.as_ref().len()).sum();
    let size = pieces_size + separator.len() * num_separators;
    let mut result = String::with_capacity(size);
    result.push_str(first.as_ref());
    for p in iter {
        result.push_str(separator);
        result.push_str(p.as_ref());
    }
    debug_assert_eq!(size, result.len());
    result
}

#[cfg(test)]
#[macro_export]
/// Utility function for parsing json. Useful in unit tests. Simply wrap the
/// json! macro in a from_value to deserialize the contents to arbitrary structs.
macro_rules! parse_json {
    ($($json:tt)+) => {
        ::serde_json::from_value(::serde_json::json!($($json)+)).expect("failed to deserialize")
    }
}

pub(crate) fn spawn_with_handle<F, O>(f: F) -> impl Future<Output = O>
where
    F: Future<Output = O> + 'static + Send,
    O: std::fmt::Debug + 'static + Send,
{
    let (tx, rx) = oneshot::channel();
    tokio::spawn(f.map(move |x| tx.send(x).unwrap()));
    async {
        rx.await.unwrap()
    }
}
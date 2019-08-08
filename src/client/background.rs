use crate::body::LiftBody;
use futures::{Future, Poll};
use http_body::Body as HttpBody;
use hyper::client::conn::Connection as HyperConnection;
use log::debug;
use std::fmt::{self, Debug};
use std::sync::{Arc, Mutex};
use tokio_io::{AsyncRead, AsyncWrite};

/// Background task for a client connection.
///
/// This type is not used directly by a user of this library,
/// but it can show up in trait bounds of generic types.
pub struct Background<T, B>
where
    T: AsyncRead + AsyncWrite + Send + 'static,
    B: HttpBody + Send + 'static,
    B::Data: Send,
    B::Error: Into<crate::Error>,
{
    connection: HyperConnection<T, LiftBody<B>>,
    handle: Handle,
}

/// Shared handle between background task and connection
#[derive(Clone, Debug, Default)]
pub(super) struct Handle {
    /// Errors encountered by the background task
    error: Arc<Mutex<Option<hyper::Error>>>,
}

impl Handle {
    pub(super) fn get_error(&self) -> Option<hyper::Error> {
        self.error
            .try_lock()
            .ok()
            .and_then(|mut err| err.take())
    }
}

impl<T, B> Background<T, B>
where
    T: AsyncRead + AsyncWrite + Send + 'static,
    B: HttpBody + Send + 'static,
    B::Data: Send,
    B::Error: Into<crate::Error>,
{
    pub(super) fn new(connection: HyperConnection<T, LiftBody<B>>) -> (Self, Handle) {
        let handle = Handle::default();
        let bg = Background {
            connection,
            handle: handle.clone(),
        };
        (bg, handle)
    }
}

impl<T, B> Future for Background<T, B>
where
    T: AsyncRead + AsyncWrite + Send + 'static,
    B: HttpBody + Send + 'static,
    B::Data: Send,
    B::Error: Into<crate::Error>,
{
    type Item = ();
    type Error = ();

    fn poll(&mut self) -> Poll<(), ()> {
        self.connection.poll().map_err(|e| {
            // errors are tracked by the handle, so lowering this
            // serverity to debug.
            debug!("error with hyper: {}", e);
            if let Ok(mut l) = self.handle.error.try_lock() {
                *l = Some(e.into());
            }
        })
    }
}

impl<T, B> Debug for Background<T, B>
where
    T: Debug + AsyncRead + AsyncWrite + Send + 'static,
    B: HttpBody + Send + 'static,
    B::Data: Send,
    B::Error: Into<crate::Error>,
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("Background")
            .field("connection", &self.connection)
            .finish()
    }
}

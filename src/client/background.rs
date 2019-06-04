use crate::body::LiftBody;
use futures::{Future, Poll};
use http_body::Body as HttpBody;
use hyper::client::conn::Connection as HyperConnection;
use log::error;
use std::fmt::{self, Debug};
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
}

impl<T, B> Background<T, B>
where
    T: AsyncRead + AsyncWrite + Send + 'static,
    B: HttpBody + Send + 'static,
    B::Data: Send,
    B::Error: Into<crate::Error>,
{
    pub(super) fn new(connection: HyperConnection<T, LiftBody<B>>) -> Self {
        Background { connection }
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
            error!("error with hyper: {}", e);
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

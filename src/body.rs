//! Tower <-> hyper body utilities

use futures::Poll;
use hyper::body::Payload;
use tower_http::Body as HttpBody;

/// Pinned body
pub type RecvBody = LiftBody<hyper::Body>;

/// Lifts a body to support `Payload` and `BufStream`
#[derive(Debug)]
pub struct LiftBody<T> {
    inner: T,
}

impl<T> LiftBody<T> {
    /// Lifts the inner `T`
    pub fn new(inner: T) -> Self {
        LiftBody { inner }
    }
}

impl<T: Payload> LiftBody<T> {
    /// Get the inner wrapped payload
    pub fn into_inner(self) -> T {
        self.inner
    }
}

impl<T: Payload> HttpBody for LiftBody<T> {
    type Item = <T as Payload>::Data;
    type Error = <T as Payload>::Error;

    fn poll_buf(&mut self) -> Poll<Option<Self::Item>, Self::Error> {
        self.inner.poll_data()
    }

    fn poll_trailers(&mut self) -> Poll<Option<hyper::HeaderMap>, Self::Error> {
        self.inner.poll_trailers()
    }

    fn is_end_stream(&self) -> bool {
        self.inner.is_end_stream()
    }
}

impl<T> Payload for LiftBody<T>
where
    T: HttpBody + Send + 'static,
    T::Item: Send,
    T::Error: Into<crate::Error>,
{
    type Data = T::Item;
    type Error = T::Error;
    fn poll_data(&mut self) -> Poll<Option<Self::Data>, Self::Error> {
        self.inner.poll_buf()
    }

    fn poll_trailers(&mut self) -> Poll<Option<hyper::HeaderMap>, Self::Error> {
        self.inner.poll_trailers()
    }
    fn is_end_stream(&self) -> bool {
        self.inner.is_end_stream()
    }
}

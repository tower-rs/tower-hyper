//! Tower <-> hyper body utilities

use futures::Poll;
use http_body::Body as HttpBody;
use hyper::body::Payload;

pub use hyper::Body;

/// Lifts a body to support `Payload`
#[derive(Debug)]
pub struct LiftBody<T> {
    inner: T,
}

impl<T: HttpBody> From<T> for LiftBody<T> {
    fn from(inner: T) -> Self {
        LiftBody { inner }
    }
}

impl<T> Payload for LiftBody<T>
where
    T: HttpBody + Send + 'static,
    T::Data: Send,
    T::Error: Into<crate::Error>,
{
    type Data = T::Data;
    type Error = T::Error;

    fn poll_data(&mut self) -> Poll<Option<Self::Data>, Self::Error> {
        self.inner.poll_data()
    }

    fn poll_trailers(&mut self) -> Poll<Option<hyper::HeaderMap>, Self::Error> {
        self.inner.poll_trailers()
    }
    fn is_end_stream(&self) -> bool {
        self.inner.is_end_stream()
    }
}

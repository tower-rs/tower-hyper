//! Tower <-> hyper body utilities

use futures::Poll;
use http_body::Body as HttpBody;
use hyper::body::Payload;

/// Specialized Body that takes a `hyper::Body` and implements `tower_http::Body`.
#[derive(Debug)]
pub struct Body {
    inner: hyper::Body,
}

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

impl From<hyper::Body> for Body {
    fn from(inner: hyper::Body) -> Self {
        Body { inner }
    }
}

impl Body {
    /// Get the inner wrapped `hyper::Body`.
    pub fn into_inner(self) -> hyper::Body {
        self.inner
    }
}

impl HttpBody for Body {
    type Data = hyper::Chunk;
    type Error = hyper::Error;

    fn poll_data(&mut self) -> Poll<Option<Self::Data>, Self::Error> {
        hyper::body::Payload::poll_data(&mut self.inner)
    }

    fn poll_trailers(&mut self) -> Poll<Option<hyper::HeaderMap>, Self::Error> {
        hyper::body::Payload::poll_trailers(&mut self.inner)
    }

    fn is_end_stream(&self) -> bool {
        hyper::body::Payload::is_end_stream(&self.inner)
    }
}

impl Payload for Body {
    type Data = hyper::Chunk;
    type Error = hyper::Error;

    fn poll_data(&mut self) -> Poll<Option<Self::Data>, Self::Error> {
        hyper::body::Payload::poll_data(&mut self.inner)
    }

    fn poll_trailers(&mut self) -> Poll<Option<hyper::HeaderMap>, Self::Error> {
        hyper::body::Payload::poll_trailers(&mut self.inner)
    }

    fn is_end_stream(&self) -> bool {
        hyper::body::Payload::is_end_stream(&self.inner)
    }
}

use crate::Body;
use futures::{Async, Future, Poll};
use hyper::Response;

/// Lift a hyper ResponseFuture to one which returns a `tower_http::Body`.
#[derive(Debug)]
pub struct ResponseFuture<F> {
    pub(super) inner: F,
}

impl<F> Future for ResponseFuture<F>
where
    F: Future<Item = Response<hyper::Body>, Error = hyper::Error>,
{
    type Item = Response<Body>;
    type Error = hyper::Error;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        match self.inner.poll() {
            Ok(futures::Async::Ready(body)) => {
                let body = body.map(Body::from);
                Ok(Async::Ready(body))
            }
            Ok(Async::NotReady) => Ok(Async::NotReady),
            Err(e) => Err(e),
        }
    }
}

use crate::body::{Body, LiftBody};
use futures::{Future, Poll};
use http::{Request, Response};
use hyper::client::conn;
use tower_http::Body as HttpBody;
use tower_service::Service;

use std::fmt;

/// The connection provided from `hyper`
///
/// This provides an interface for `DirectService` that will
/// drive the inner service via `poll_service` and can send
/// requests via `call`.
#[derive(Debug)]
pub struct Connection<B>
where
    B: HttpBody,
{
    sender: conn::SendRequest<LiftBody<B>>,
}

impl<B> Connection<B>
where
    B: HttpBody,
{
    pub(super) fn new(sender: conn::SendRequest<LiftBody<B>>) -> Self {
        Connection { sender }
    }
}

impl<B> Service<Request<B>> for Connection<B>
where
    B: HttpBody + Send + 'static,
    B::Item: Send,
    B::Error: Into<crate::Error>,
{
    type Response = Response<Body>;
    type Error = hyper::Error;
    type Future = ResponseFuture<conn::ResponseFuture>;

    fn poll_ready(&mut self) -> Poll<(), Self::Error> {
        self.sender.poll_ready()
    }

    fn call(&mut self, req: Request<B>) -> Self::Future {
        ResponseFuture(self.sender.send_request(req.map(LiftBody::from)))
    }
}

/// Lift a hyper ResponseFuture to one which returns LiftBody
pub struct ResponseFuture<F>(pub(crate) F);

impl<F> Future for ResponseFuture<F>
where
    F: Future<Item = Response<hyper::Body>, Error = hyper::Error>,
{
    type Item = Response<Body>;
    type Error = hyper::Error;

    #[inline]
    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        match self.0.poll() {
            Ok(futures::Async::Ready(body)) => Ok(futures::Async::Ready(body.map(Body::from))),
            Ok(futures::Async::NotReady) => Ok(futures::Async::NotReady),
            Err(e) => Err(e),
        }
    }
}

impl<F: fmt::Debug> fmt::Debug for ResponseFuture<F> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "ResponseFuture<{:?}>", self.0)
    }
}

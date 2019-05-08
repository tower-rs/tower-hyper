use super::ResponseFuture;
use crate::body::{Body, LiftBody};
use futures::Poll;
use http::{Request, Response};
use http_body::Body as HttpBody;
use hyper::client::conn;
use tower_service::Service;

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
    B::Data: Send,
    B::Error: Into<crate::Error>,
{
    type Response = Response<Body>;
    type Error = hyper::Error;
    type Future = ResponseFuture<conn::ResponseFuture>;

    fn poll_ready(&mut self) -> Poll<(), Self::Error> {
        self.sender.poll_ready()
    }

    fn call(&mut self, req: Request<B>) -> Self::Future {
        let inner = self.sender.send_request(req.map(LiftBody::from));
        ResponseFuture { inner }
    }
}

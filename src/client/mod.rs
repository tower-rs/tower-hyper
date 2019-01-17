mod connect;
mod connection;

pub use self::connect::Connect;
pub use self::connection::Connection;

use futures::{Async, Poll};
use http::{Request, Response};
use hyper::{
    body::Payload, client::connect::Connect as HyperConnect, client::ResponseFuture, Body,
    Client as HyperClient,
};
use tower_direct_service::DirectService;
use tower_service::Service;

pub struct Client<C, B> {
    inner: HyperClient<C, B>,
}

impl<C, B> Client<C, B> {
    pub fn new(inner: HyperClient<C, B>) -> Self {
        Self { inner }
    }
}

impl<C, B> DirectService<Request<B>> for Client<C, B>
where
    C: HyperConnect + Sync + 'static,
    C::Transport: 'static,
    C::Future: 'static,
    B: Payload + Send + 'static,
    B::Data: Send,
{
    type Response = Response<Body>;
    type Error = hyper::Error;
    type Future = ResponseFuture;

    fn poll_service(&mut self) -> Poll<(), Self::Error> {
        Ok(Async::Ready(()))
    }

    fn poll_close(&mut self) -> Poll<(), Self::Error> {
        Ok(Async::Ready(()))
    }

    fn poll_ready(&mut self) -> Poll<(), Self::Error> {
        Ok(Async::Ready(()))
    }

    fn call(&mut self, req: Request<B>) -> Self::Future {
        let (parts, body) = req.into_parts();
        let req = Request::from_parts(parts, body.into());
        self.inner.request(req)
    }
}

impl<C, B> Service<Request<B>> for Client<C, B>
where
    C: HyperConnect + Sync + 'static,
    C::Transport: 'static,
    C::Future: 'static,
    B: Payload + Send + 'static,
    B::Data: Send,
{
    type Response = Response<Body>;
    type Error = hyper::Error;
    type Future = ResponseFuture;

    fn poll_ready(&mut self) -> Poll<(), Self::Error> {
        Ok(Async::Ready(()))
    }

    fn call(&mut self, req: Request<B>) -> Self::Future {
        let (parts, body) = req.into_parts();
        let req = Request::from_parts(parts, body.into());
        self.inner.request(req)
    }
}

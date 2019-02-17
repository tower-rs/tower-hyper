//! The client portion of `tower-hyper`.
//!
//! The client module contains three main utiliteies client, connection
//! and connect. Connection and Connect are designed to be used together
//! where as Client is a thicker client designed to be used by itself. There
//! is less control over driving the inner service compared to Connection. The
//! other difference is that Connection is a lowerlevel connection, so there is no
//! connection pooling etc, that is the job of the services that wrap it.

mod connect;
mod connection;

pub use self::connect::{Connect, ConnectError};
pub use self::connection::Connection;
pub use hyper::client::conn::Builder;

use futures::{Async, Poll};
use http::{Request, Response};
use hyper::{
    body::Payload, client::connect::Connect as HyperConnect, client::ResponseFuture, Body,
};
use tower_service::Service;

/// The client wrapp for `hyper::Client`
///
/// The generics `C` and `B` are 1-1 with the generic
/// types within `hyper::Client`.
pub struct Client<C, B> {
    inner: hyper::Client<C, B>,
}

impl<C, B> Client<C, B> {
    /// Create a new client from a `hyper::Client`
    pub fn new(inner: hyper::Client<C, B>) -> Self {
        Self { inner }
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

    /// Poll to see if the service is ready, since `hyper::Client`
    /// already handles this internally this will always return ready
    fn poll_ready(&mut self) -> Poll<(), Self::Error> {
        Ok(Async::Ready(()))
    }

    /// Send the sepcficied request to the inner `hyper::Client`
    fn call(&mut self, req: Request<B>) -> Self::Future {
        self.inner.request(req)
    }
}

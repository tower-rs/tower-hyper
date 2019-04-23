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
mod future;

pub use self::connect::{Connect, ConnectError};
pub use self::connection::Connection;
use self::future::ResponseFuture;
pub use hyper::client::conn::Builder;

use crate::body::{Body, LiftBody};
use futures::{Async, Poll};
use hyper::{
    client::connect::Connect as HyperConnect,
    client::{self, HttpConnector},
    Request, Response,
};
use tower_http::Body as HttpBody;
use tower_service::Service;

/// The client wrapp for `hyper::Client`
///
/// The generics `C` and `B` are 1-1 with the generic
/// types within `hyper::Client`.
#[derive(Clone, Debug)]
pub struct Client<C, B> {
    inner: hyper::Client<C, LiftBody<B>>,
}

impl<B> Client<HttpConnector, B>
where
    B: HttpBody + Send + 'static,
    B::Item: Send,
    B::Error: Into<crate::Error>,
{
    /// Create a new client, using the default hyper settings
    pub fn new() -> Self {
        let inner = hyper::Client::builder().build_http();
        Client { inner }
    }
}

impl<C, B> Client<C, B> {
    /// Create a new client by providing the inner `hyper::Client`
    ///
    /// ## Example
    ///
    /// The existing default is:
    ///```
    ///   use http::Request;
    ///   use tower_hyper::client::Client;
    ///   use tower_service::Service;
    ///
    ///   let inner = hyper::Client::builder().build_http();
    ///   let mut client = Client::with_client(inner);
    ///   let _ = client.call(Request::new(vec![0, 1, 2]));
    /// ````
    /// which returns a `Client<HttpConnector, B>` for any B: `HttpBody`.
    pub fn with_client(inner: hyper::Client<C, LiftBody<B>>) -> Self {
        Self { inner }
    }
}

impl<C, B> Service<Request<B>> for Client<C, B>
where
    C: HyperConnect + Sync + 'static,
    C::Transport: 'static,
    C::Future: 'static,
    B: HttpBody + Send + 'static,
    B::Item: Send,
    B::Error: Into<crate::Error>,
{
    type Response = Response<Body>;
    type Error = hyper::Error;
    type Future = ResponseFuture<client::ResponseFuture>;

    /// Poll to see if the service is ready, since `hyper::Client`
    /// already handles this internally this will always return ready
    fn poll_ready(&mut self) -> Poll<(), Self::Error> {
        Ok(Async::Ready(()))
    }

    /// Send the sepcficied request to the inner `hyper::Client`
    fn call(&mut self, req: Request<B>) -> Self::Future {
        let inner = self.inner.request(req.map(LiftBody::from));
        ResponseFuture { inner }
    }
}

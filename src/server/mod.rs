//! The server porition of tower hyper

use crate::body::{Body, LiftBody};
use futures::{try_ready, Future, Poll};
use hyper::service::Service as HyperService;
use hyper::{Request, Response};
use std::fmt;
use std::marker::PhantomData;
use tokio_io::{AsyncRead, AsyncWrite};
use tower::MakeService;
use tower_http::Body as HttpBody;
use tower_http::HttpService;
use tower_service::Service;

pub use hyper::server::conn::Http;

/// A stream mapping incoming IOs to new services.
pub type Serve<E> = Box<Future<Item = (), Error = Error<E>> + Send + 'static>;

/// Server implemenation for hyper
#[derive(Debug)]
pub struct Server<S, B> {
    maker: S,
    _pd: PhantomData<B>,
}

/// Error's produced by a `Connection`.
#[derive(Debug)]
pub enum Error<E> {
    /// Error's originating from `hyper`.
    Protocol(hyper::Error),
    /// Error's produced from creating the inner service.
    MakeService(E),
}

#[derive(Debug)]
struct LiftService<T, B> {
    inner: T,
    _pd: PhantomData<B>,
}

#[derive(Debug)]
struct LiftServiceFuture<F, B> {
    inner: F,
    _pd: PhantomData<B>,
}

impl<S, B> Server<S, B>
where
    S: MakeService<(), Request<Body>, Response = Response<B>> + Send + 'static,
    S::MakeError: Into<crate::Error>,
    S::Error: Into<crate::Error>,
    S::Future: Send,
    S::Service: Service<Request<Body>> + Send,
    <S::Service as Service<Request<Body>>>::Future: Send + 'static,
    B: HttpBody + Send + 'static,
    B::Item: Send + 'static,
    B::Error: Into<crate::Error> + 'static,
{
    /// Create a new server from a `MakeService`
    pub fn new(maker: S) -> Self {
        Server {
            maker,
            _pd: PhantomData,
        }
    }

    /// Serve the `io` stream via default hyper http settings
    pub fn serve<I>(&mut self, io: I) -> Serve<S::MakeError>
    where
        I: AsyncRead + AsyncWrite + Send + 'static,
    {
        let http = Http::new();
        self.serve_with(io, http)
    }

    /// Serve the `io` stream via the provided hyper http settings
    pub fn serve_with<I>(&mut self, io: I, http: Http) -> Serve<S::MakeError>
    where
        I: AsyncRead + AsyncWrite + Send + 'static,
    {
        let fut = self
            .maker
            .make_service(())
            .map_err(Error::MakeService)
            .and_then(move |svc| {
                let svc = LiftService::new(svc);
                http.serve_connection(io, svc).map_err(Error::Protocol)
            });

        Box::new(fut)
    }
}

impl<T, B> LiftService<T, B> {
    pub fn new(inner: T) -> Self {
        LiftService {
            inner,
            _pd: PhantomData,
        }
    }
}

// Lift takes in a service on `LiftBody<Body> -> B` (both implement http::Body)
// and returns a `hyper::Service` which instead takes in `Request<Body>`
// and outputs `Response<LiftBody<B>>` (which both implement payload).
impl<T, B> HyperService for LiftService<T, B>
where
    B: HttpBody + Send + 'static,
    B::Item: Send,
    B::Error: Into<crate::Error>,
    T: HttpService<Body, ResponseBody = B>,
    T::Error: Into<crate::Error>,
    // T::Future: Send + 'static,
{
    type ReqBody = hyper::Body;
    type ResBody = LiftBody<B>;
    type Error = crate::Error;
    type Future = LiftServiceFuture<T::Future, B>;

    fn poll_ready(&mut self) -> Poll<(), Self::Error> {
        self.inner.poll_ready().map_err(Into::into)
    }

    fn call(&mut self, request: Request<Self::ReqBody>) -> Self::Future {
        let fut = self.inner.call(request.map(Body::from));

        LiftServiceFuture {
            inner: fut,
            _pd: PhantomData,
        }
    }
}

impl<F, B> Future for LiftServiceFuture<F, B>
where
    F: Future<Item = Response<B>>,
    F::Error: Into<crate::Error>,
    B: HttpBody + Send,
    B::Item: Send,
    B::Error: Into<crate::Error>,
{
    type Item = Response<LiftBody<B>>;
    type Error = crate::Error;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        let response = try_ready!(self.inner.poll().map_err(Into::into));
        Ok(response.map(LiftBody::from).into())
    }
}

impl<E: fmt::Debug> fmt::Display for Error<E> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match &self {
            Error::Protocol(why) => f.debug_tuple("Protocol").field(why).finish(),
            Error::MakeService(why) => f.debug_tuple("MakeService").field(why).finish(),
        }
    }
}

impl<E: fmt::Debug> ::std::error::Error for Error<E> {}

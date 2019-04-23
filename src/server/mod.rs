//! The server porition of tower hyper

use crate::body::{Body, LiftBody};
use futures::{try_ready, Future, Poll};
use hyper::server::conn::Connection;
use hyper::service::Service as HyperService;
use hyper::{Request, Response};
use std::fmt;
use std::marker::PhantomData;
use tokio_executor::DefaultExecutor;
use tokio_io::{AsyncRead, AsyncWrite};
use tower::MakeService;
use tower_http::Body as HttpBody;
use tower_http::HttpService;
use tower_service::Service;

pub use hyper::server::conn::Http;

/// Server implemenation for hyper
#[derive(Debug)]
pub struct Server<S, B> {
    maker: S,
    _pd: PhantomData<B>,
}

/// The future that represents the connection.
pub struct Serve<S, B, I>
where
    S: MakeService<(), Request<Body>, Response = Response<B>>,
    S::Service: Send,
    S::MakeError: Into<crate::Error> + 'static,
    S::Error: Into<crate::Error> + 'static,
    S::Future: Send + 'static,
    S::Service: Service<Request<Body>> + Send + 'static,
    <S::Service as Service<Request<Body>>>::Future: Send + 'static,
    B: HttpBody + Send + 'static,
    B::Item: Send,
    B::Error: Into<crate::Error>,
{
    state: State<S::Future, Connection<I, LiftService<S::Service, B>, DefaultExecutor>>,
    http: Http<DefaultExecutor>,
    io: Option<I>,
}

enum State<M, S> {
    Make(M),
    Call(S),
}

#[derive(Debug)]
struct LiftService<T, B> {
    inner: T,
    _pd: PhantomData<B>,
}

impl<S, B> Server<S, B>
where
    S: MakeService<(), Request<Body>, Response = Response<B>>,
    S::MakeError: Into<crate::Error>,
    S::Error: Into<crate::Error>,
    S::Future: Send,
    S::Service: Service<Request<Body>> + Send,
    <S::Service as Service<Request<Body>>>::Future: Send,
    B: HttpBody + Send,
    B::Item: Send,
    B::Error: Into<crate::Error>,
{
    /// Create a new server from a `MakeService`
    pub fn new(maker: S) -> Self {
        Server {
            maker,
            _pd: PhantomData,
        }
    }

    /// Serve the `io` stream via default hyper http settings
    pub fn serve<I>(&mut self, io: I) -> Serve<S, B, I>
    where
        I: AsyncRead + AsyncWrite + 'static,
    {
        let http = Http::new().with_executor(DefaultExecutor::current());
        self.serve_with(io, http)
    }

    /// Serve the `io` stream via the provided hyper http settings
    pub fn serve_with<I>(&mut self, io: I, http: Http<DefaultExecutor>) -> Serve<S, B, I>
    where
        I: AsyncRead + AsyncWrite + 'static,
    {
        let mk = self.maker.make_service(());
        let state = State::Make(mk);
        let io = Some(io);
        Serve { state, http, io }
    }
}

impl<S, B, I> Future for Serve<S, B, I>
where
    S: MakeService<(), Request<Body>, Response = Response<B>>,
    S::MakeError: Into<crate::Error>,
    S::Error: Into<crate::Error>,
    S::Future: Send,
    S::Service: Service<Request<Body>> + Send,
    <S::Service as Service<Request<Body>>>::Future: Send,
    B: HttpBody + Send,
    B::Item: Send,
    B::Error: Into<crate::Error>,
    I: AsyncRead + AsyncWrite + 'static,
{
    type Item = ();
    type Error = crate::Error;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        loop {
            match &mut self.state {
                State::Make(fut) => {
                    let svc = try_ready!(fut.poll().map_err(Into::into));
                    let service = LiftService::new(svc);
                    let io = self.io.take().unwrap();
                    let fut = self.http.serve_connection(io, service);
                    self.state = State::Call(fut);
                    continue;
                }
                State::Call(fut) => return fut.poll().map_err(Into::into),
            }
        }
    }
}

impl<S, B, I> fmt::Debug for Serve<S, B, I>
where
    S: MakeService<(), Request<Body>, Response = Response<B>>,
    S::Service: Send,
    S::MakeError: Into<crate::Error> + 'static,
    S::Error: Into<crate::Error> + 'static,
    S::Future: Send + 'static,
    S::Service: Service<Request<Body>> + Send + 'static,
    <S::Service as Service<Request<Body>>>::Future: Send + 'static,
    B: HttpBody + Send + 'static,
    B::Item: Send,
    B::Error: Into<crate::Error>,
    I: AsyncRead + AsyncWrite + 'static,
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Serve<S, B, I>")
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
    T: HttpService<Body, ResponseBody = B> + Send + 'static,
    T::Error: Into<crate::Error> + 'static,
    T::Future: Send + 'static,
{
    type ReqBody = hyper::Body;
    type ResBody = LiftBody<B>;
    type Error = crate::Error;
    type Future = Box<Future<Item = Response<Self::ResBody>, Error = Self::Error> + Send + 'static>;

    fn call(&mut self, request: Request<Self::ReqBody>) -> Self::Future {
        let fut = self
            .inner
            .call(request.map(Body::from))
            .map(|r| r.map(LiftBody::from))
            .map_err(Into::into);

        Box::new(fut)
    }
}

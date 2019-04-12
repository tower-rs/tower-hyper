//! The server porition of tower hyper

use crate::body::LiftBody;
use futures::Future;
use hyper::Body;
use hyper::service::Service as HyperService;
use hyper::{Request, Response};
use tokio_io::{AsyncRead, AsyncWrite};
use tower::MakeService;
use tower_http::HttpService;
use tower_http::Body as HttpBody;
use tower_service::Service;

pub use hyper::server::conn::Http;

/// Server implemenation for hyper
#[derive(Debug)]
pub struct Server<S, B> {
    maker: S,
    _marker: std::marker::PhantomData<B>,
}

impl<S, B> Server<S, B>
where
    B: HttpBody + Send + 'static,
    B::Item: Send,
    B::Error: Into<crate::Error>,
    S: MakeService<(), Request<LiftBody<Body>>, Response = Response<B>> + Send + 'static,
    S::Error: Into<crate::Error> + 'static,
    S::Future: Send + 'static,
    S::Service: Service<Request<LiftBody<Body>>> + Send + 'static,
    <S::Service as Service<Request<LiftBody<Body>>>>::Future: Send + 'static,
{
    /// Create a new server from a `MakeService`
    pub fn new(maker: S) -> Self {
        Server {
            maker,
            _marker: std::marker::PhantomData
        }
    }

    /// Serve the `io` stream via default hyper http settings
    pub fn serve<I>(
        &mut self,
        io: I,
    ) -> Box<Future<Item = (), Error = hyper::Error> + Send + 'static>
    where
        I: AsyncRead + AsyncWrite + Send + 'static,
    {
        let http = Http::new();
        self.serve_with(io, http)
    }

    /// Serve the `io` stream via the provided hyper http settings
    pub fn serve_with<I>(
        &mut self,
        io: I,
        http: Http,
    ) -> Box<Future<Item = (), Error = hyper::Error> + Send + 'static>
    where
        I: AsyncRead + AsyncWrite + Send + 'static,
    {
        let fut = self
            .maker
            .make_service(())
            .map_err(|_| unimplemented!())
            .and_then(move |service| {
                let service = Lift::new(service);
                http.serve_connection::<Lift<S::Service, B>, I, LiftBody<B>>(io, service)
            });

        Box::new(fut)
    }
}

struct Lift<T, B> {
    inner: T,
    _marker: std::marker::PhantomData<B>,
}

impl<T, B> Lift<T, B> {
    pub fn new(inner: T) -> Self {
        Lift { 
            inner,
            _marker: std::marker::PhantomData,
        }
    }
}

// Lift takes in a service on `LiftBody<Body> -> B` (both implement http::Body)
// and returns a `hyper::Service` which instead takes in `Request<Body>`
// and outputs `Response<LiftBody<B>>` (which both implement payload).
impl<T, B> HyperService for Lift<T, B>
where
    B: HttpBody + Send + 'static,
    B::Item: Send,
    B::Error: Into<crate::Error>,
    T: HttpService<LiftBody<Body>, ResponseBody = B> + Send + 'static,
    T::Error: Into<crate::Error> + 'static,
    T::Future: Send + 'static,
{
    type ReqBody = Body;
    type ResBody = LiftBody<B>;
    type Error = hyper::Error;
    // type Future = T::Future;
    type Future = Box<Future<Item = Response<Self::ResBody>, Error = Self::Error> + Send + 'static>;

    fn call(&mut self, request: Request<Self::ReqBody>) -> Self::Future {
        let fut = self
            .inner
            .call(request.map(LiftBody::new))
            .map(|r| r.map(LiftBody::new))
            .map_err(|_e| {
                unimplemented!()
            });

        Box::new(fut)
    }
}

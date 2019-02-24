//! The server porition of tower hyper

use futures::Future;
use hyper::service::Service as HyperService;
use hyper::Body;
use hyper::{Request, Response};
use tokio_io::{AsyncRead, AsyncWrite};
use tower_http::HttpService;
use tower_service::Service;
use tower_util::MakeService;

pub use hyper::server::conn::Http;

/// Server implemenation for hyper
#[derive(Debug)]
pub struct Server<S> {
    maker: S,
}

impl<S> Server<S>
where
    S: MakeService<(), Request<Body>, Response = Response<Body>> + Send + 'static,
    S::Error: std::error::Error + Send + Sync + 'static,
    S::Future: Send + 'static,
    S::Service: Send + 'static,
    <S::Service as Service<Request<Body>>>::Future: Send + 'static,
{
    /// Create a new server from a `MakeService`
    pub fn new(maker: S) -> Self {
        Server { maker }
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
                http.serve_connection(io, service)
            });

        Box::new(fut)
    }
}

struct Lift<T> {
    inner: T,
}

impl<T> Lift<T> {
    pub fn new(inner: T) -> Self {
        Lift { inner }
    }
}

impl<T> HyperService for Lift<T>
where
    T: HttpService<Body, ResponseBody = Body> + Send + 'static,
    T::Error: std::error::Error + Send + Sync + 'static,
    T::Future: Send + 'static,
{
    type ReqBody = Body;
    type ResBody = Body;
    type Error = hyper::Error;
    // type Future = T::Future;
    type Future = Box<Future<Item = Response<Self::ResBody>, Error = Self::Error> + Send + 'static>;

    fn call(&mut self, request: Request<Self::ReqBody>) -> Self::Future {
        let fut = self.inner.call(request).map_err(|_| unimplemented!());

        Box::new(fut)
    }
}

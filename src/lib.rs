mod client;
mod util;
use futures::{future, Async, Poll};
use http::{Request, Response};
use hyper::{
    body::Payload,
    client::connect::Connect as HyperConnect,
    client::{HttpConnector, ResponseFuture},
    Body, Client as HyperClient,
};
use tower_retry::Policy;
use tower_service::Service;

pub use self::client::{Connect, Connection};

pub struct Client<C = HttpConnector, B = Body> {
    inner: HyperClient<C, B>,
}

impl<C, B> Client<C, B> {
    fn new(inner: HyperClient<C, B>) -> Self {
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

    fn poll_ready(&mut self) -> Poll<(), Self::Error> {
        Ok(Async::Ready(()))
    }

    fn call(&mut self, req: Request<B>) -> Self::Future {
        let (parts, body) = req.into_parts();
        let req = Request::from_parts(parts, body.into());
        self.inner.request(req)
    }
}

#[derive(Clone)]
pub struct RetryPolicy {
    attempts: u8,
}

impl RetryPolicy {
    fn new(attempts: u8) -> Self {
        RetryPolicy { attempts }
    }
}

impl<T> Policy<Request<T>, Response<Body>, hyper::Error> for RetryPolicy
where
    T: Into<Body> + Clone,
{
    type Future = future::FutureResult<Self, ()>;

    fn retry(
        &self,
        _: &Request<T>,
        result: Result<&Response<Body>, &hyper::Error>,
    ) -> Option<Self::Future> {
        if self.attempts == 0 {
            // We ran out of retries, hence us returning none.
            return None;
        }

        match result {
            Ok(res) => {
                if res.status().is_server_error() {
                    let policy = RetryPolicy::new(self.attempts - 1);
                    Some(future::ok(policy))
                } else {
                    // 2xx-4xx shouldn't be retried.
                    None
                }
            }
            Err(_) => Some(future::ok(RetryPolicy {
                attempts: self.attempts - 1,
            })),
        }
    }

    fn clone_request(&self, req: &Request<T>) -> Option<Request<T>> {
        // there is no .parts(&self) method on request.
        let body = req.body().clone();
        let mut clone = http::Request::new(body);
        *clone.uri_mut() = req.uri().clone();
        *clone.headers_mut() = req.headers().clone();
        *clone.method_mut() = req.method().clone();
        *clone.method_mut() = req.method().clone();
        *clone.version_mut() = req.version().clone();
        Some(clone)
    }
}

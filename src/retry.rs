//! Provides retry based utilities
use crate::body::LiftBody;
use futures::{future, Async, Poll};
use http::{Request, Response};
use hyper::body::Chunk;
use std::marker::PhantomData;
use tower_http::Body as HttpBody;
use tower_retry::Policy;

/// A simple retry policy for hyper bases requests.
///
/// It currently only retries any non `4xx-2xx` responses. To use
/// this policy you must use the `Body` type that is provided in this
/// crate.
#[derive(Debug)]
pub struct RetryPolicy<E> {
    attempts: u8,
    _pd: PhantomData<E>,
}

/// Trait to perform attempted clones of the requests body.
pub trait TryClone: Sized {
    /// Attempt to clone
    fn try_clone(&self) -> Option<Self>;
}

impl<C> TryClone for C
    where C: Clone
{
    fn try_clone(&self) -> Option<Self> {
        Some(self.clone())
    }
}

/// A specialized Body for hyper
///
/// This provides a simple workaround Body to allow the
/// request to be cloned. This is mostly important with the
/// `tower-retry` middleware. This is because on retry it must
/// clone the request and thus the `Body`. This `Body` only allows one
/// to construct it from a single `hyper::body::Chunk`. This allows it to
/// enforce `Clone`.
#[derive(Debug)]
pub struct Body<B> {
    pub(crate) body: Option<B>,
}

impl<E> RetryPolicy<E> {
    /// Create a new policy with the provided amount of retries
    pub fn new(attempts: u8) -> Self {
        RetryPolicy {
            attempts,
            _pd: PhantomData,
        }
    }
}

impl<T, E> Policy<Request<T>, Response<LiftBody<hyper::Body>>, E> for RetryPolicy<E>
where
    T: HttpBody + TryClone
{
    type Future = future::FutureResult<Self, ()>;

    fn retry(
        &self,
        _: &Request<T>,
        result: Result<&Response<LiftBody<hyper::Body>>, &E>,
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
                _pd: PhantomData,
            })),
        }
    }

    fn clone_request(&self, req: &Request<T>) -> Option<Request<T>> {
        match req.body().try_clone() {
            Some(body) => {
                let mut clone = http::Request::new(body);
                *clone.uri_mut() = req.uri().clone();
                *clone.headers_mut() = req.headers().clone();
                *clone.method_mut() = req.method().clone();
                *clone.method_mut() = req.method().clone();
                *clone.version_mut() = req.version();
                Some(clone)
            }
            None => None,
        }
    }
}

impl<E> Clone for RetryPolicy<E> {
    fn clone(&self) -> RetryPolicy<E> {
        RetryPolicy {
            attempts: self.attempts,
            _pd: PhantomData,
        }
    }
}

impl<B> HttpBody for Body<B>
where
    B: Into<Chunk> + Send + 'static,
{
    type Item = hyper::body::Chunk;
    type Error = hyper::Error;

    fn poll_buf(&mut self) -> Poll<Option<Self::Item>, Self::Error> {
        match self.body.take() {
            Some(body) => {
                let body = Some(body.into());
                Ok(Async::Ready(body))
            }

            None => Ok(Async::Ready(None)),
        }
    }

    fn poll_trailers(&mut self) -> Poll<Option<http::HeaderMap>, Self::Error> {
        Ok(Async::Ready(None))
    }
}

impl<B> From<B> for Body<B>
where
    B: Into<Chunk>,
{
    fn from(b: B) -> Body<B> {
        Body { body: Some(b) }
    }
}

impl<B> TryClone for Body<B>
where
    B: Into<Chunk> + Clone,
{
    fn try_clone(&self) -> Option<Body<B>> {
        match self.body {
            Some(ref body) => Some(Body {
                body: Some(body.clone()),
            }),
            None => None,
        }
    }
}

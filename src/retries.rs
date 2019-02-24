use futures::future;
use http::{Request, Response};
use hyper::Body;
use std::marker::PhantomData;
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

impl<E> RetryPolicy<E> {
    /// Create a new policy with the provided amount of retries
    pub fn new(attempts: u8) -> Self {
        RetryPolicy {
            attempts,
            _pd: PhantomData,
        }
    }
}

impl<T, E> Policy<Request<T>, Response<Body>, E> for RetryPolicy<E>
where
    T: Into<Body> + TryClone,
{
    type Future = future::FutureResult<Self, ()>;

    fn retry(&self, _: &Request<T>, result: Result<&Response<Body>, &E>) -> Option<Self::Future> {
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
                *clone.version_mut() = req.version().clone();
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

/// Trait to perform attempted clones of the requests body.
pub trait TryClone: Sized {
    /// Attempt to clone
    fn try_clone(&self) -> Option<Self>;
}

use futures::{Future, Poll};
use hyper::client::connect::{Connect, Destination};
use tokio_io::{AsyncRead, AsyncWrite};
use tower_direct_service::DirectService;
use tower_service::Service;

/// The ConnectService trait is used to create transports
///
/// The goal of this service is to allow composable methods to creating
/// `AsyncRead + AsyncWrite` transports. This could mean creating a TLS
/// based connection or using some other method to authenticate the connection.
pub trait ConnectService<A> {
    type Response: AsyncRead + AsyncWrite;
    type Error;
    type Future: Future<Item = Self::Response, Error = Self::Error>;

    fn connect(&mut self, target: A) -> Self::Future;
}

// Here for references
// impl<A, C> ConnectService<A> for C
// where
//     C: Service<A>,
//     C::Response: AsyncRead + AsyncWrite,
// {
//     type Response = C::Response;
//     type Error = C::Error;
//     type Future = C::Future;

//     fn connect(&mut self, target: A) -> Self::Future {
//         self.call(target)
//     }
// }

impl<C> ConnectService<Destination> for C
where
    C: Connect,
    C::Future: 'static,
{
    type Response = C::Transport;
    type Error = C::Error;
    type Future = Box<Future<Item = Self::Response, Error = Self::Error> + Send + 'static>;

    fn connect(&mut self, req: Destination) -> Self::Future {
        let fut = <Self as Connect>::connect(self, req).map(|(transport, _)| transport);
        Box::new(fut)
    }
}

/// Creates new `Service` values.
///
/// Acts as a service factory. This is useful for cases where new `Service`
/// values must be produced. One case is a TCP servier listener. The listner
/// accepts new TCP streams, obtains a new `Service` value using the
/// `MakeService` trait, and uses that new `Service` value to process inbound
/// requests on that new TCP stream.
///
/// This is essentially a trait alias for a `Service` of `Service`s.
pub trait MakeService<Target, Request>: self::sealed::Sealed<Target, Request> {
    /// Responses given by the service
    type Response;

    /// Errors produced by the service
    type Error;

    /// The `Service` value created by this factory
    type Service: DirectService<Request, Response = Self::Response, Error = Self::Error>;

    /// Errors produced while building a service.
    type MakeError;

    /// The future of the `Service` instance.
    type Future: Future<Item = Self::Service, Error = Self::MakeError>;

    /// Returns `Ready` when the factory is able to process create more services.
    ///
    /// If the service is at capacity, then `NotReady` is returned and the task
    /// is notified when the service becomes ready again. This function is
    /// expected to be called while on a task.
    ///
    /// This is a **best effort** implementation. False positives are permitted.
    /// It is permitted for the service to return `Ready` from a `poll_ready`
    /// call and the next invocation of `call` results in an error.
    fn poll_ready(&mut self) -> Poll<(), Self::MakeError>;

    /// Create and return a new service value asynchronously.
    fn make_service(&mut self, target: Target) -> Self::Future;
}

impl<M, S, Target, Request> self::sealed::Sealed<Target, Request> for M
where
    M: Service<Target, Response = S>,
    S: DirectService<Request>,
{
}

impl<M, S, Target, Request> MakeService<Target, Request> for M
where
    M: Service<Target, Response = S>,
    S: DirectService<Request>,
{
    type Response = S::Response;
    type Error = S::Error;
    type Service = S;
    type MakeError = M::Error;
    type Future = M::Future;

    fn poll_ready(&mut self) -> Poll<(), Self::MakeError> {
        Service::poll_ready(self)
    }

    fn make_service(&mut self, target: Target) -> Self::Future {
        Service::call(self, target)
    }
}

mod sealed {
    pub trait Sealed<A, B> {}
}

use futures::{Future, Poll, Async, try_ready};
use hyper::client::connect::{Connect, Destination};
use tower_direct_service::DirectService;
use tower_service::Service;

/// A bridge between `hyper::client::connect::Connect` types
/// and `tower_util::MakeConnection`.
///
/// # Example
///
/// ```
/// # use tower_hyper::util::Connector;
/// # use tower_hyper::client::{Connect, Builder};
/// # use hyper::client::HttpConnector;
/// let connector = Connector::new(HttpConnector::new(1));
/// let mut hyper = Connect::new(connector, Builder::new());
/// # let hyper: Connect<hyper::client::connect::Destination, hyper::Body, Connector<HttpConnector>> = hyper;
/// ```
pub struct Connector<C> {
    inner: C,
}

/// The future that resolves to the eventual inner transport
/// as built by `hyper::client::connect::Connect`.
pub struct ConnectorFuture<C>
where
    C: Connect,
{
    inner: C::Future,
}

impl<C> Connector<C>
where
    C: Connect,
{
    /// Construct a new connector from a `hyper::client::connect::Connect`
    pub fn new(inner: C) -> Self {
        Self { inner }
    }
}

impl<C> Service<Destination> for Connector<C>
where
    C: Connect,
{
    type Response = C::Transport;
    type Error = C::Error;
    type Future = ConnectorFuture<C>;

    fn poll_ready(&mut self) -> Poll<(), Self::Error> {
        Ok(().into())
    }

    fn call(&mut self, target: Destination) -> Self::Future {
        let fut = self.inner.connect(target);
        ConnectorFuture { inner: fut }
    }
}

impl<C> Future for ConnectorFuture<C>
where
    C: Connect,
{
    type Item = C::Transport;
    type Error = C::Error;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        let (transport, _) = try_ready!(self.inner.poll());

        Ok(Async::Ready(transport))
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

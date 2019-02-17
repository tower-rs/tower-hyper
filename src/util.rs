use futures::{Future, Poll, Async, try_ready};
use hyper::client::connect::{Connect, Destination};
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

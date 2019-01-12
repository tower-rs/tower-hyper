use super::Connection;
use crate::util::ConnectService;
use futures::{try_ready, Async, Future, Poll};
use hyper::body::Payload;
use hyper::client::conn::{Builder, Handshake};
use hyper::Error;
use std::marker::PhantomData;
use tower_service::Service;

pub struct Connect<A, B, C> {
    inner: C,
    builder: Builder,
    _pd: PhantomData<(A, B)>,
}

pub struct ConnectFuture<A, B, C>
where
    B: Payload,
    C: ConnectService<A>,
{
    state: State<A, B, C>,
    builder: Builder,
}

enum State<A, B, C>
where
    B: Payload,
    C: ConnectService<A>,
{
    Connect(C::Future),
    Handshake(Handshake<C::Response, B>),
}

#[derive(Debug)]
pub enum ConnectError<T> {
    Connect(T),
    Handshake(Error),
}

// ===== impl Connect =====

impl<A, B, C> Connect<A, B, C>
where
    C: ConnectService<A>,
    B: Payload,
    C::Response: Send + 'static,
{
    /// Create a new `Connect`.
    ///
    /// The `connect` argument is used to obtain new session layer instances
    /// (`AsyncRead` + `AsyncWrite`). For each new client service returned, a
    /// task will be spawned onto `executor` that will be used to manage the Hyper
    /// connection.
    pub fn new(inner: C, builder: Builder) -> Self {
        Connect {
            inner,
            builder,
            _pd: PhantomData,
        }
    }
}

impl<A, B, C> Service<A> for Connect<A, B, C>
where
    C: ConnectService<A> + 'static,
    B: Payload + 'static,
    C::Response: Send + 'static,
{
    type Response = Connection<C::Response, B>;
    type Error = ConnectError<C::Error>;
    type Future = ConnectFuture<A, B, C>;

    fn poll_ready(&mut self) -> Poll<(), Self::Error> {
        Ok(().into())
    }

    /// Obtains a Connection on a single plaintext h2 connection to a remote.
    fn call(&mut self, target: A) -> Self::Future {
        let state = State::Connect(self.inner.connect(target));
        let builder = self.builder.clone();

        ConnectFuture { state, builder }
    }
}

// ===== impl ConnectFuture =====

impl<A, B, C> Future for ConnectFuture<A, B, C>
where
    C: ConnectService<A>,
    B: Payload,
    C::Response: Send + 'static,
{
    type Item = Connection<C::Response, B>;
    type Error = ConnectError<C::Error>;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        loop {
            let io = match self.state {
                State::Connect(ref mut fut) => {
                    let res = fut.poll().map_err(ConnectError::Connect);

                    try_ready!(res)
                }
                State::Handshake(ref mut fut) => {
                    let (sender, conn) = try_ready!(fut.poll().map_err(ConnectError::Handshake));

                    let connection = Connection::new(sender, conn);

                    return Ok(Async::Ready(connection));
                }
            };

            let handshake = self.builder.handshake(io);
            self.state = State::Handshake(handshake);
        }
    }
}

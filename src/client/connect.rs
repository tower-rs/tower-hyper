use super::Connection;
use crate::util::ConnectService;
use futures::{try_ready, Async, Future, Poll};
use hyper::body::Payload;
use hyper::client::conn::{Builder, Handshake};
use hyper::Error;
use std::marker::PhantomData;
use std::fmt;
use std::error::Error as StdError;
use tower_service::Service;

/// Creates a `hyper` connection
///
/// This accepts a `hyper::client::conn::Builder` and provides
/// a `MakeService` implementation to create connections from some
/// target `A`
pub struct Connect<A, B, C> {
    inner: C,
    builder: Builder,
    _pd: PhantomData<(A, B)>,
}

/// The future thre represents the eventual connection
/// or error
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

/// The error produced from creating a connection
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
    /// The `C` argument is used to obtain new session layer instances
    /// (`AsyncRead` + `AsyncWrite`). For each new client service returned, a
    /// DirectService is returned that can be driven by `poll_service` and to send
    /// requests via `call`.
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

    /// This always returns ready
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

// ==== impl ConnectError ====
impl<T> fmt::Display for ConnectError<T>
where
    T: fmt::Display,
{
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        match *self {
            ConnectError::Connect(ref why) => write!(
                f,
                "Error attempting to establish underlying session layer: {}",
                why
            ),
            ConnectError::Handshake(ref why) => {
                write!(f, "Error while performing HTTP handshake: {}", why,)
            }
        }
    }
}

impl<T> StdError for ConnectError<T>
where
    T: StdError,
{
    fn description(&self) -> &str {
        match *self {
            ConnectError::Connect(_) =>
                "error attempting to establish underlying session layer",
            ConnectError::Handshake(_) =>
                "error performing HTTP handshake"
        }
    }

    fn cause(&self) -> Option<&StdError> {
        match *self {
            ConnectError::Connect(ref why) => Some(why),
            ConnectError::Handshake(ref why) => Some(why),
        }
    }
}

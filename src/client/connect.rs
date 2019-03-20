use super::Connection;
use futures::future::Executor;
use futures::{try_ready, Async, Future, Poll};
use hyper::body::Payload;
use hyper::client::conn::{Builder, Handshake};
use hyper::Error;
use log::error;
use std::error::Error as StdError;
use std::fmt;
use std::marker::PhantomData;
use tokio_executor::DefaultExecutor;
use tower::MakeConnection;
use tower_service::Service;

/// Creates a `hyper` connection
///
/// This accepts a `hyper::client::conn::Builder` and provides
/// a `MakeService` implementation to create connections from some
/// target `A`
#[derive(Debug)]
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
    C: MakeConnection<A>,
{
    state: State<A, B, C>,
    builder: Builder,
}

enum State<A, B, C>
where
    B: Payload,
    C: MakeConnection<A>,
{
    Connect(C::Future),
    Handshake(Handshake<C::Connection, B>),
}

/// The error produced from creating a connection
#[derive(Debug)]
pub enum ConnectError<T> {
    /// An error occurred while attempting to establish the connection.
    Connect(T),
    /// An error occurred while performing hyper's handshake.
    Handshake(Error),
    /// An error occurred attempting to spawn the connect task on the
    /// provided executor.
    SpawnError,
}

// ===== impl Connect =====

impl<A, B, C> Connect<A, B, C>
where
    C: MakeConnection<A>,
    B: Payload,
    C::Connection: Send + 'static,
{
    /// Create a new `Connect`.
    ///
    /// The `C` argument is used to obtain new session layer instances
    /// (`AsyncRead` + `AsyncWrite`). For each new client service returned, a
    /// Service is returned that can be driven by `poll_service` and to send
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
    C: MakeConnection<A> + 'static,
    B: Payload + 'static,
    C::Connection: Send + 'static,
{
    type Response = Connection<B>;
    type Error = ConnectError<C::Error>;
    type Future = ConnectFuture<A, B, C>;

    /// Check if the `MakeConnection` is ready for a new connection.
    fn poll_ready(&mut self) -> Poll<(), Self::Error> {
        self.inner
            .poll_ready()
            .map_err(|e| ConnectError::Connect(e))
    }

    /// Obtains a Connection on a single plaintext h2 connection to a remote.
    fn call(&mut self, target: A) -> Self::Future {
        let state = State::Connect(self.inner.make_connection(target));
        let builder = self.builder.clone();

        ConnectFuture { state, builder }
    }
}

// ===== impl ConnectFuture =====

impl<A, B, C> Future for ConnectFuture<A, B, C>
where
    C: MakeConnection<A>,
    B: Payload,
    C::Connection: Send + 'static,
{
    type Item = Connection<B>;
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

                    let exec = DefaultExecutor::current();
                    exec.execute(conn.map_err(|e| error!("error with hyper: {}", e)))
                        .map_err(|_| ConnectError::SpawnError)?;

                    let connection = Connection::new(sender);

                    return Ok(Async::Ready(connection));
                }
            };

            let handshake = self.builder.handshake(io);
            self.state = State::Handshake(handshake);
        }
    }
}

impl<A, B, C> fmt::Debug for ConnectFuture<A, B, C>
where
    C: MakeConnection<A>,
    B: Payload,
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str("ConnectFuture")
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
            ConnectError::SpawnError => write!(f, "Error spawning background task"),
        }
    }
}

impl<T> StdError for ConnectError<T>
where
    T: StdError,
{
    fn description(&self) -> &str {
        match *self {
            ConnectError::Connect(_) => "error attempting to establish underlying session layer",
            ConnectError::Handshake(_) => "error performing HTTP handshake",
            ConnectError::SpawnError => "Error spawning background task",
        }
    }

    fn cause(&self) -> Option<&StdError> {
        match *self {
            ConnectError::Connect(ref why) => Some(why),
            ConnectError::Handshake(ref why) => Some(why),
            ConnectError::SpawnError => None,
        }
    }
}

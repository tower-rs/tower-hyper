use super::Connection;
use crate::body::LiftBody;
use futures::{future::MapErr, try_ready, Async, Future, Poll};
use http::Version;
use http_body::Body as HttpBody;
use http_connection::HttpConnection;
use hyper::body::Payload;
use hyper::client::conn::{Builder, Connection as HyperConnection, Handshake};
use hyper::Error;
use log::error;
use std::fmt;
use std::marker::PhantomData;
use tokio_executor::{DefaultExecutor, TypedExecutor};
use tokio_io::{AsyncRead, AsyncWrite};
use tower_http_util::connection::HttpMakeConnection;
use tower_service::Service;

/// Creates a `hyper` connection
///
/// This accepts a `hyper::client::conn::Builder` and provides
/// a `MakeService` implementation to create connections from some
/// target `A`
#[derive(Debug)]
pub struct Connect<A, B, C, E> {
    inner: C,
    builder: Builder,
    exec: E,
    _pd: PhantomData<(A, B)>,
}

/// Executor that will spawn the background connection task.
pub trait ConnectExecutor<T, B>:
    TypedExecutor<MapErr<HyperConnection<T, B>, fn(hyper::Error) -> ()>>
where
    T: AsyncRead + AsyncWrite + Send + 'static,
    B: Payload + 'static,
{
}

/// The future thre represents the eventual connection
/// or error
pub struct ConnectFuture<A, B, C, E>
where
    B: HttpBody,
    C: HttpMakeConnection<A>,
{
    state: State<A, B, C>,
    builder: Builder,
    exec: E,
}

enum State<A, B, C>
where
    B: HttpBody,
    C: HttpMakeConnection<A>,
{
    Connect(C::Future),
    Handshake(Handshake<C::Connection, LiftBody<B>>),
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

// ==== impl ConnectExecutor ====

impl<E, T, B> ConnectExecutor<T, B> for E
where
    T: AsyncRead + AsyncWrite + Send + 'static,
    B: Payload + 'static,
    E: TypedExecutor<MapErr<HyperConnection<T, B>, fn(hyper::Error) -> ()>>,
{
}

// ===== impl Connect =====

impl<A, B, C> Connect<A, B, C, DefaultExecutor>
where
    C: HttpMakeConnection<A>,
    B: HttpBody,
    C::Connection: Send + 'static,
{
    /// Create a new `Connect`.
    ///
    /// The `C` argument is used to obtain new session layer instances
    /// (`AsyncRead` + `AsyncWrite`). For each new client service returned, a
    /// Service is returned that can be driven by `poll_service` and to send
    /// requests via `call`.
    pub fn new(inner: C) -> Self {
        Connect::with_builder(inner, Builder::new())
    }

    /// Create a new `Connect` with a builder.
    pub fn with_builder(inner: C, builder: Builder) -> Self {
        Connect::with_executor(inner, builder, DefaultExecutor::current())
    }
}

impl<A, B, C, E> Connect<A, B, C, E>
where
    C: HttpMakeConnection<A>,
    B: HttpBody,
    C::Connection: Send + 'static,
{
    /// Create a new `Connect`.
    ///
    /// The `C` argument is used to obtain new session layer instances
    /// (`AsyncRead` + `AsyncWrite`). For each new client service returned, a
    /// Service is returned that can be driven by `poll_service` and to send
    /// requests via `call`.
    ///
    /// The `E` argument is the executor that the background task for the connection
    /// will be spawned on.
    pub fn with_executor(inner: C, builder: Builder, exec: E) -> Self {
        Connect {
            inner,
            builder,
            exec,
            _pd: PhantomData,
        }
    }
}

impl<A, B, C, E> Service<A> for Connect<A, B, C, E>
where
    C: HttpMakeConnection<A> + 'static,
    B: HttpBody + Send + 'static,
    B::Data: Send,
    B::Error: Into<crate::Error>,
    C::Connection: Send + 'static,
    E: ConnectExecutor<C::Connection, LiftBody<B>> + Clone,
{
    type Response = Connection<B>;
    type Error = ConnectError<C::Error>;
    type Future = ConnectFuture<A, B, C, E>;

    /// Check if the `MakeConnection` is ready for a new connection.
    fn poll_ready(&mut self) -> Poll<(), Self::Error> {
        self.inner.poll_ready().map_err(ConnectError::Connect)
    }

    /// Obtains a Connection on a single plaintext h2 connection to a remote.
    fn call(&mut self, target: A) -> Self::Future {
        let state = State::Connect(self.inner.make_connection(target));
        let builder = self.builder.clone();
        let exec = self.exec.clone();

        ConnectFuture {
            state,
            builder,
            exec,
        }
    }
}

// ===== impl ConnectFuture =====

impl<A, B, C, E> Future for ConnectFuture<A, B, C, E>
where
    C: HttpMakeConnection<A>,
    B: HttpBody + Send + 'static,
    B::Data: Send,
    B::Error: Into<crate::Error>,
    C::Connection: Send + 'static,
    E: ConnectExecutor<C::Connection, LiftBody<B>>,
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

                    self.exec
                        .spawn(conn.map_err(|e| error!("error with hyper: {}", e)))
                        .map_err(|_| ConnectError::SpawnError)?;

                    let connection = Connection::new(sender);

                    return Ok(Async::Ready(connection));
                }
            };

            let mut builder = self.builder.clone();

            if let Some(Version::HTTP_2) = io.negotiated_version() {
                builder.http2_only(true);
            }

            let handshake = builder.handshake(io);

            self.state = State::Handshake(handshake);
        }
    }
}

impl<A, B, C, E> fmt::Debug for ConnectFuture<A, B, C, E>
where
    C: HttpMakeConnection<A>,
    B: HttpBody,
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

impl<T> std::error::Error for ConnectError<T>
where
    T: std::error::Error,
{
    fn description(&self) -> &str {
        match *self {
            ConnectError::Connect(_) => "error attempting to establish underlying session layer",
            ConnectError::Handshake(_) => "error performing HTTP handshake",
            ConnectError::SpawnError => "Error spawning background task",
        }
    }

    fn cause(&self) -> Option<&std::error::Error> {
        match *self {
            ConnectError::Connect(ref why) => Some(why),
            ConnectError::Handshake(ref why) => Some(why),
            ConnectError::SpawnError => None,
        }
    }
}

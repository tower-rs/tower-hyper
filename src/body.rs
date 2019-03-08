//! Tower <-> hyper body utilities

use futures::Poll;
use hyper::body::Payload;
use tokio_buf::BufStream;

/// Lifts a body to support `Payload` and `BufStream`
#[derive(Debug)]
pub struct LiftBody<T> {
    inner: T,
}

impl<T> LiftBody<T> {
    /// Lifts the inner `T`
    pub fn new(inner: T) -> Self {
        LiftBody { inner }
    }
}

impl<T: Payload> BufStream for LiftBody<T> {
    type Item = <T as Payload>::Data;
    type Error = <T as Payload>::Error;

    fn poll_buf(&mut self) -> Poll<Option<Self::Item>, Self::Error> {
        Payload::poll_data(self)
    }
}

impl<T: Payload> Payload for LiftBody<T> {
    type Data = <T as Payload>::Data;
    type Error = <T as Payload>::Error;

    fn poll_data(&mut self) -> Poll<Option<Self::Data>, Self::Error> {
        Payload::poll_data(self)
    }
}

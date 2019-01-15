use futures::Future;
use hyper::client::connect::{Connect, Destination};
use tokio_io::{AsyncRead, AsyncWrite};
// use tower_service::Service;

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

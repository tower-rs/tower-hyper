use futures::{future, Future, Poll, Stream};
use hyper::{Request, Response};
use tokio_tcp::TcpListener;
use tower_hyper::body::Body;
use tower_hyper::server::Server;
use tower_service::Service;

fn main() {
    pretty_env_logger::init();

    let addr = "127.0.0.1:3000".parse().unwrap();
    let bind = TcpListener::bind(&addr).expect("bind");

    println!("Listening on http://{}", addr);

    let server = Server::new(MakeSvc);

    let server = bind
        .incoming()
        .fold(server, |mut server, stream| {
            if let Err(e) = stream.set_nodelay(true) {
                return Err(e);
            }

            hyper::rt::spawn(
                server
                    .serve(stream)
                    .map_err(|e| panic!("Server error {:?}", e)),
            );

            Ok(server)
        })
        .map_err(|e| panic!("serve error: {:?}", e))
        .map(|_| {});

    hyper::rt::run(future::lazy(|| server));
}

struct Svc;
impl Service<Request<Body>> for Svc {
    type Response = Response<Body>;
    type Error = hyper::Error;
    type Future = future::FutureResult<Self::Response, Self::Error>;

    fn poll_ready(&mut self) -> Poll<(), Self::Error> {
        Ok(().into())
    }

    fn call(&mut self, req: Request<Body>) -> Self::Future {
        let body = req.into_body();
        let res = Response::new(Body::from(body));
        future::ok(res)
    }
}

struct MakeSvc;
impl Service<()> for MakeSvc {
    type Response = Svc;
    type Error = hyper::Error;
    type Future = future::FutureResult<Self::Response, Self::Error>;

    fn poll_ready(&mut self) -> Poll<(), Self::Error> {
        Ok(().into())
    }

    fn call(&mut self, _: ()) -> Self::Future {
        future::ok(Svc)
    }
}

use futures::{future, Future, Poll, Stream};
use hyper::{Body, Request, Response};
use tokio_tcp::TcpListener;
use tower_hyper::body::LiftBody;
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
impl Service<Request<LiftBody<Body>>> for Svc {
    type Response = Response<LiftBody<Body>>;
    type Error = hyper::Error;
    type Future = future::FutureResult<Self::Response, Self::Error>;

    fn poll_ready(&mut self) -> Poll<(), Self::Error> {
        Ok(().into())
    }

    fn call(&mut self, _req: Request<LiftBody<Body>>) -> Self::Future {
        let body = LiftBody::new(Body::from("Hello World!"));
        let res = Response::new(body);
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

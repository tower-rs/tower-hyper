use futures::{future, Future, Stream};
use http::{Request, Uri};
use hyper::client::conn::Builder;
use hyper::client::connect::{Destination, HttpConnector};
use hyper::rt;
use hyper::Body;
use tower_buffer::Buffer;
use tower_hyper::Connect;
use tower_service::Service;
use tower_util::MakeService;

fn main() {
    pretty_env_logger::init();
    rt::run(future::lazy(|| {
        let dst = Destination::new(Uri::from_static("http://127.0.0.1:3000"));
        let mut hyper = Connect::new(HttpConnector::new(1), Builder::new());

        hyper
            .make_service(dst)
            .map_err(|err| eprintln!("Connect Error {:?}", err))
            .and_then(|conn| Buffer::new_direct(conn, 1).map_err(|_| panic!("Unable to spawn!")))
            .and_then(|mut conn| {
                conn.call(Request::new(Body::empty()))
                    .map_err(|e| e.into_inner().expect("Buffer closed"))
                    .and_then(|response| {
                        println!("Response Status: {:?}", response.status());
                        response.into_body().concat2()
                    })
                    .and_then(|body| {
                        println!("Response Body: {:?}", body);
                        Ok(())
                    })
                    .map_err(|err| eprintln!("Request Error: {}", err))
            })
    }));
}

use futures::{future, Future};
use http::{Request, Uri};
use hyper::client::connect::{Destination, HttpConnector};
use hyper::rt;
use tokio_buf::util::BufStreamExt;
use tower::MakeService;
use tower::{Service, ServiceExt};
use tower_http::BodyExt;
use tower_hyper::client::Connect;
use tower_hyper::util::Connector;

fn main() {
    pretty_env_logger::init();
    rt::run(future::lazy(|| {
        let dst = Destination::try_from_uri(Uri::from_static("http://127.0.0.1:3000")).unwrap();
        let connector = Connector::new(HttpConnector::new(1));
        let mut hyper = Connect::new(connector);

        hyper
            .make_service(dst)
            .map_err(|err| eprintln!("Connect Error {:?}", err))
            .and_then(|conn| {
                conn.ready()
                    .and_then(|mut conn| conn.call(Request::new(Vec::new())))
                    .map_err(|e| eprintln!("Call Error: {}", e))
                    .and_then(|response| {
                        println!("Response Status: {:?}", response.status());
                        response
                            .into_body()
                            .into_buf_stream()
                            .collect::<Vec<u8>>()
                            .map(|v| String::from_utf8(v).unwrap())
                            .map_err(|e| eprintln!("Body Error: {:?}", e))
                    })
                    .and_then(|body| {
                        println!("Response Body: {}", body);
                        Ok(())
                    })
            })
    }));
}

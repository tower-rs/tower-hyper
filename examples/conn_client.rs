use futures::{future, Future};
use http::{Request, Uri};
use hyper::client::conn::Builder;
use hyper::client::connect::{Destination, HttpConnector};
use hyper::rt;
use tokio_buf::util::BufStreamExt;
use tower::MakeService;
use tower_buffer::Buffer;
use tower_http::BodyExt;
use tower_hyper::client::Connect;
use tower_hyper::util::Connector;
use tower_service::Service;

fn main() {
    pretty_env_logger::init();
    rt::run(future::lazy(|| {
        let dst = Destination::try_from_uri(Uri::from_static("http://127.0.0.1:3000")).unwrap();
        let connector = Connector::new(HttpConnector::new(1));
        let mut hyper = Connect::new(connector, Builder::new());

        hyper
            .make_service(dst)
            .map_err(|err| eprintln!("Connect Error {:?}", err))
            .and_then(|conn| Ok(Buffer::new(conn, 1)))
            .and_then(|mut conn| {
                conn.call(Request::new(Vec::new())) // Empty request, but `Vec<u8>` implements `tower_http::Body`
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

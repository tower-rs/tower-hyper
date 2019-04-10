use futures::{future, Future};
use http::{Method, Request, Uri};
use hyper::rt;
use tokio_buf::util::BufStreamExt;
use tower_http::BodyExt;
use tower_hyper::client::Client;
use tower_service::Service;

fn main() {
    rt::run(future::lazy(|| {
        let uri = "http://httpbin.org/ip".parse::<Uri>().unwrap();
        let mut svc = Client::new();

        let request = Request::builder()
            .uri(uri)
            .method(Method::GET)
            .body(Vec::new())
            .unwrap();

        svc.call(request)
            .map_err(|err| eprintln!("Request Error: {}", err))
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
    }));
}

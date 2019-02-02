use futures::{future, Future, Stream};
use http::{Method, Request, Uri};
use hyper::rt;
use hyper::Body;
use tower_hyper::client::Client;
use tower_service::Service;

fn main() {
    rt::run(future::lazy(|| {
        let uri = "http://httpbin.org/ip".parse::<Uri>().unwrap();
        let mut svc = Client::new(hyper::Client::new());

        let request = Request::builder()
            .uri(uri)
            .method(Method::GET)
            .body(Body::empty())
            .unwrap();

        svc.call(request)
            .and_then(|response| {
                println!("Response Status: {:?}", response.status());
                response.into_body().concat2()
            })
            .and_then(|body| {
                println!("Response Body: {:?}", body);
                Ok(())
            })
            .map_err(|err| eprintln!("Request Error: {}", err))
    }));
}

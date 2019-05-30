use futures::Future;
use hyper::{Body, Request};
use tokio::runtime::Runtime;
use tower_hyper::client::Client;
use tower_service::Service;

mod support;
use support::*;

#[test]
fn hello_world() {
    let mut rt = Runtime::new().unwrap();
    let addr = next_addr();
    rt.spawn(server(addr, false));

    let mut client = Client::new();

    let req = Request::get(format!("http://{}", addr))
        .body(Body::empty())
        .unwrap();

    let fut = client.call(req).and_then(|res| {
        assert_eq!(res.status(), http::StatusCode::OK);
        Ok(())
    });

    rt.block_on(fut).unwrap();
    rt.shutdown_now().wait().unwrap()
}

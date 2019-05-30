use futures::Future;
use hyper::service::service_fn_ok;
use hyper::{Body, Response, Server};
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::sync::atomic::{AtomicUsize, Ordering};

pub fn server(addr: SocketAddr, http2_only: bool) -> impl Future<Item = (), Error = ()> {
    let make_service = || service_fn_ok(|_req| Response::new(Body::from("Hello World")));

    Server::bind(&addr)
        .http2_only(http2_only)
        .serve(make_service)
        .map_err(|e| panic!("{}", e))
}

static NEXT_PORT: AtomicUsize = AtomicUsize::new(1234);
pub fn next_addr() -> SocketAddr {
    let port = NEXT_PORT.fetch_add(1, Ordering::AcqRel) as u16;
    SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), port)
}

use futures::{Future, Poll};
use http::{Uri, Version};
use http_connection::HttpConnection;
use hyper::client::connect::{Destination, HttpConnector};
use hyper::{Body, Request};
use std::net::SocketAddr;
use tokio::runtime::Runtime;
use tokio_tcp::TcpStream;
use tower_hyper::client::Connect;
use tower_hyper::util::Connector;
use tower_service::Service;
use tower_util::MakeService;

mod support;
use support::*;

#[test]
fn hello_world() {
    let mut rt = Runtime::new().unwrap();

    let addr = next_addr();
    rt.spawn(server(addr, false));

    let connector = Connector::new(HttpConnector::new(1));
    let mut connect = Connect::new(connector);

    let req = Request::get(format!("http://{}", addr))
        .body(Body::empty())
        .unwrap();

    let uri = format!("http://{}", addr).parse::<Uri>().unwrap();
    let dst = Destination::try_from_uri(uri).unwrap();
    let mut client = rt.block_on(connect.make_service(dst)).unwrap();

    let fut = client.call(req).and_then(|res| {
        assert_eq!(res.status(), http::StatusCode::OK);
        Ok(())
    });

    rt.block_on(fut).unwrap();
    rt.shutdown_now().wait().unwrap()
}

#[test]
fn http_connection_http2() {
    let mut rt = Runtime::new().unwrap();

    let addr = next_addr();
    rt.spawn(server(addr, true));

    let mut connect = Connect::new(Http2);

    let req = Request::get(format!("http://{}", addr))
        .body(Body::empty())
        .unwrap();

    let mut client = rt.block_on(connect.make_service(addr)).unwrap();

    let fut = client.call(req).and_then(|res| {
        assert_eq!(res.status(), http::StatusCode::OK);
        Ok(())
    });

    rt.block_on(fut).unwrap();
    rt.shutdown_now().wait().unwrap()
}

struct Http2;

struct Stream(TcpStream);

impl Service<SocketAddr> for Http2 {
    type Response = Stream;
    type Error = std::io::Error;
    type Future = Box<dyn Future<Item = Self::Response, Error = Self::Error> + Send + 'static>;

    fn poll_ready(&mut self) -> Poll<(), Self::Error> {
        Ok(().into())
    }

    fn call(&mut self, req: SocketAddr) -> Self::Future {
        Box::new(TcpStream::connect(&req).map(|s| Stream(s)))
    }
}

impl std::io::Read for Stream {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        self.0.read(buf)
    }
}

impl std::io::Write for Stream {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.0.write(&buf)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.0.flush()
    }
}

impl tokio::io::AsyncRead for Stream {}
impl tokio::io::AsyncWrite for Stream {
    fn shutdown(&mut self) -> Poll<(), tokio::io::Error> {
        <TcpStream as tokio::io::AsyncWrite>::shutdown(&mut self.0)
    }
}
impl HttpConnection for Stream {
    fn negotiated_version(&self) -> Option<Version> {
        Some(Version::HTTP_2)
    }
}

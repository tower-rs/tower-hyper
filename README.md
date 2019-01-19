# Tower Hyper

A (WIP) integration between hyper and tower

[![Build Status](https://travis-ci.org/tower-rs/tower-hyper.svg?branch=master)](https://travis-ci.org/tower-rs/tower-hyper)

## Example

Simple client connection example, check it out [here](/examples/conn_client.rs)

``` rust
let mut hyper = Connect::new(HttpConnector::new(1), Builder::new());

let request = hyper
	.make_service(dst)
	.and_then(|mut conn| {
		conn.call(Request::new(Body::empty()))
	})
	.and_then(|response| {
		// do something with response...
	});
	
hyper::rt::run(request);
```

`

## License

This project is licensed under the MIT license.

## Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in `tower-hyper` by you, shall be licensed as MIT, without any additional terms or conditions.



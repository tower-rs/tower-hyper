# Tower Hyper

A (WIP) integration between hyper and tower

[![Build Status](https://travis-ci.org/tower-rs/tower-hyper.svg?branch=master)](https://travis-ci.org/tower-rs/tower-hyper)

## Example

Simple client connection example, check it out [here](/examples/conn_client.rs)

``` rust
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
```

`

## License

This project is licensed under the MIT license.

## Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in `tower-hyper` by you, shall be licensed as MIT, without any additional terms or conditions.



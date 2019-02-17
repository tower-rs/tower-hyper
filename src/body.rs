use crate::retries::TryClone;
use futures::{Async, Poll};
use hyper::body::{Chunk, Payload};

#[derive(Debug)]
pub struct Body<B> {
    pub(crate) body: Option<B>,
}

impl<B> Payload for Body<B>
where
    B: Into<Chunk> + Send + 'static,
{
    type Data = hyper::body::Chunk;
    type Error = hyper::Error;

    fn poll_data(&mut self) -> Poll<Option<Self::Data>, Self::Error> {
        match self.body.take() {
            Some(body) => {
                let body = Some(body.into());
                Ok(Async::Ready(body))
            }

            None => Ok(Async::Ready(None)),
        }
    }

    fn poll_trailers(&mut self) -> Poll<Option<http::HeaderMap>, Self::Error> {
        Ok(Async::Ready(None))
    }
}

impl<B> From<B> for Body<B>
where
    B: Into<Chunk>,
{
    fn from(b: B) -> Body<B> {
        Body { body: Some(b) }
    }
}

impl<B> Into<hyper::Body> for Body<B>
where
    B: Into<Chunk> + Sized,
{
    fn into(self) -> hyper::Body {
        hyper::Body::from(self.body.unwrap().into())
    }
}

impl<B> TryClone for Body<B>
where
    B: Into<Chunk> + Clone,
{
    fn try_clone(&self) -> Option<Body<B>> {
        match self.body {
            Some(ref body) => Some(Body {
                body: Some(body.clone()),
            }),
            None => None,
        }
    }
}

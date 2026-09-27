use super::{BODY_TIMEOUT, State};
use axum::body::{Body, Bytes};
use http_body::{Body as HttpBody, Frame, SizeHint};
use http_body_util::BodyExt;
use std::{
    future::Future,
    io,
    pin::Pin,
    sync::{Arc, OnceLock},
    task::{Context, Poll},
};
use tokio::{
    sync::OwnedSemaphorePermit,
    time::{Instant, Sleep},
};

#[derive(Clone, Copy, Debug)]
pub(super) enum InputFailure {
    Timeout,
    TooLarge,
    Invalid,
    Read,
}
impl InputFailure {
    pub fn response(self) -> axum::http::Response<Body> {
        use axum::http::StatusCode as Status;
        let (status, message) = match self {
            Self::Timeout => (Status::REQUEST_TIMEOUT, "HTTP body deadline"),
            Self::TooLarge => (Status::PAYLOAD_TOO_LARGE, "HTTP body byte budget"),
            Self::Invalid => (
                Status::BAD_REQUEST,
                "HTTP body must be unambiguous UTF-8 JSON",
            ),
            Self::Read => (Status::BAD_REQUEST, "HTTP body could not be read"),
        };
        super::error(status, message)
    }
}
async fn read_input(mut body: Body) -> Result<Bytes, InputFailure> {
    let mut bytes = Vec::new();
    while let Some(frame) = body.frame().await {
        let frame = frame.map_err(|_| InputFailure::Read)?;
        if let Ok(data) = frame.into_data() {
            if data.len() > crate::transport::INPUT_BYTES - bytes.len() {
                return Err(InputFailure::TooLarge);
            }
            bytes.extend_from_slice(&data);
        }
    }
    let text = std::str::from_utf8(&bytes).map_err(|_| InputFailure::Invalid)?;
    let _: serde_json::Value = mo_common::from_json_str(text).map_err(|_| InputFailure::Invalid)?;
    Ok(Bytes::from(bytes))
}
pub(super) fn validated_input(body: Body) -> (Body, Arc<OnceLock<InputFailure>>) {
    let failure = Arc::new(OnceLock::new());
    let record = failure.clone();
    let checked = Body::from_stream(futures::stream::once(async move {
        let result = tokio::time::timeout(BODY_TIMEOUT, read_input(body))
            .await
            .unwrap_or(Err(InputFailure::Timeout));
        result.map_err(|reason| {
            let _ = record.set(reason);
            io::Error::other("HTTP input rejected")
        })
    }));
    (checked, failure)
}
/// Keep admission alive while the caller drains a bounded response. Dropping
/// the inner SSE body propagates the SDK's request cancellation signal.
pub(super) struct Output {
    inner: Option<Body>,
    permit: Option<OwnedSemaphorePermit>,
    owner: Option<Arc<State>>,
    remaining: usize,
    deadline: Pin<Box<Sleep>>,
}
impl Output {
    pub fn new(
        inner: Body,
        permit: OwnedSemaphorePermit,
        owner: Arc<State>,
        remaining: usize,
        deadline: Instant,
    ) -> Self {
        Self {
            inner: Some(inner),
            permit: Some(permit),
            owner: Some(owner),
            remaining,
            deadline: Box::pin(tokio::time::sleep_until(deadline)),
        }
    }
    fn close(&mut self) {
        self.inner.take();
        self.permit.take();
        self.owner.take();
    }
}
impl HttpBody for Output {
    type Data = Bytes;
    type Error = io::Error;
    fn poll_frame(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Option<Result<Frame<Bytes>, io::Error>>> {
        if self.inner.is_none() {
            return Poll::Ready(None);
        }
        if self.deadline.as_mut().poll(cx).is_ready() {
            self.close();
            return Poll::Ready(Some(Err(io::Error::new(
                io::ErrorKind::TimedOut,
                "HTTP response deadline",
            ))));
        }
        match Pin::new(self.inner.as_mut().expect("live body")).poll_frame(cx) {
            Poll::Ready(Some(Ok(frame))) => {
                if let Some(bytes) = frame.data_ref() {
                    if bytes.len() > self.remaining {
                        self.close();
                        return Poll::Ready(Some(Err(io::Error::other(
                            "HTTP response byte budget",
                        ))));
                    }
                    self.remaining -= bytes.len();
                }
                Poll::Ready(Some(Ok(frame)))
            }
            Poll::Ready(Some(Err(e))) => {
                self.close();
                Poll::Ready(Some(Err(io::Error::other(e))))
            }
            Poll::Ready(None) => {
                self.close();
                Poll::Ready(None)
            }
            Poll::Pending => Poll::Pending,
        }
    }
    fn is_end_stream(&self) -> bool {
        self.inner.is_none()
    }
    fn size_hint(&self) -> SizeHint {
        SizeHint::default()
    }
}

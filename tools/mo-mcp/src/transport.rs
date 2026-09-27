//! Bounded stdio transport using the SDK service, without an unbounded line or
//! response queue. Budget exhaustion closes the connection, never drops work.
use futures::{StreamExt, future::BoxFuture};
use rmcp::{RoleServer, model::*, transport::Transport};
use std::{
    collections::{HashMap, HashSet},
    io,
    sync::{
        Arc, Mutex, Weak,
        atomic::{AtomicUsize, Ordering},
    },
};
use tokio::io::{AsyncRead, AsyncWrite};
use tokio_util::{codec::FramedRead, sync::CancellationToken};
mod decode;
mod framing;
mod output;
#[cfg(test)]
mod tests;

pub const INPUT_BYTES: usize = 4 * 1024 * 1024;
pub const OUTPUT_BYTES: usize = 16 * 1024 * 1024;
pub const REQUESTS: usize = 8;
pub const NOTIFICATIONS: usize = 8;
pub const CANCEL_HISTORY: usize = 1024;

#[derive(Clone)]
struct RequestLife(Arc<()>);
struct Pending {
    life: Weak<()>,
    cancelled: bool,
    responding: bool,
}
#[derive(Default)]
struct Accounting {
    requests: HashMap<RequestId, Pending>,
    retired: HashSet<RequestId>,
    error: Option<&'static str>,
}
pub struct Observer {
    accounting: Mutex<Accounting>,
    notifications: Arc<AtomicUsize>,
    stopped: CancellationToken,
}
impl Observer {
    /// Connection termination is an execution-lifetime signal for the thin
    /// adapter. Legacy durable hosts deliberately keep their old job semantics.
    pub async fn disconnected(&self) {
        self.stopped.cancelled().await;
    }
    fn fail(&self, message: &'static str) {
        self.accounting.lock().unwrap().error.get_or_insert(message);
        self.stopped.cancel();
    }
    pub fn error(&self) -> Option<&'static str> {
        self.accounting.lock().unwrap().error
    }
    fn admit(&self, request: &mut JsonRpcRequest<ClientRequest>) -> io::Result<()> {
        let mut state = self.accounting.lock().unwrap();
        let finished: Vec<_> = state
            .requests
            .iter()
            .filter(|(_, p)| p.cancelled && !p.responding && p.life.strong_count() == 0)
            .map(|(id, _)| id.clone())
            .collect();
        for id in finished {
            state.requests.remove(&id);
            state.retired.insert(id);
        }
        if state.retired.len() > CANCEL_HISTORY
            || state.requests.len() >= REQUESTS
            || state.requests.contains_key(&request.id)
            || state.retired.contains(&request.id)
            || serde_json::to_vec(&request.id)
                .map_err(io::Error::other)?
                .len()
                > 256
        {
            return Err(invalid("request identity or in-flight budget"));
        }
        let life = RequestLife(Arc::new(()));
        state.requests.insert(
            request.id.clone(),
            Pending {
                life: Arc::downgrade(&life.0),
                cancelled: false,
                responding: false,
            },
        );
        request.request.extensions_mut().insert(life);
        Ok(())
    }
}
struct NotificationLife(Arc<AtomicUsize>);
impl Drop for NotificationLife {
    fn drop(&mut self) {
        self.0.fetch_sub(1, Ordering::AcqRel);
    }
}

pub struct BoundedTransport<R, W> {
    read: FramedRead<R, framing::Frames>,
    write: Arc<tokio::sync::Mutex<W>>,
    observer: Arc<Observer>,
    // Owned by the transport, not by one cancellable receive future. The SDK
    // may select another event after any partial write; resume the same frame.
    protocol_error: Option<BoxFuture<'static, io::Result<()>>>,
}
impl<R: AsyncRead, W> BoundedTransport<R, W> {
    pub fn new(read: R, write: W) -> (Self, Arc<Observer>) {
        let observer = Arc::new(Observer {
            accounting: Mutex::new(Accounting::default()),
            notifications: Arc::new(AtomicUsize::new(0)),
            stopped: CancellationToken::new(),
        });
        (
            Self {
                read: FramedRead::new(read, framing::Frames::default()),
                write: Arc::new(tokio::sync::Mutex::new(write)),
                observer: observer.clone(),
                protocol_error: None,
            },
            observer,
        )
    }
}
fn invalid(message: &'static str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message)
}
impl<R, W> Transport<RoleServer> for BoundedTransport<R, W>
where
    R: AsyncRead + Unpin + Send + 'static,
    W: AsyncWrite + Unpin + Send + 'static,
{
    type Error = io::Error;
    fn send(
        &mut self,
        item: ServerJsonRpcMessage,
    ) -> impl Future<Output = io::Result<()>> + Send + 'static {
        let writer = self.write.clone();
        let observer = self.observer.clone();
        // Claim synchronously, before the SDK schedules the write future. A
        // late cancellation must not retire a response already being sent.
        let id = match &item {
            JsonRpcMessage::Response(r) => Some(r.id.clone()),
            JsonRpcMessage::Error(e) => e.id.clone(),
            _ => None,
        };
        let associated = id.as_ref().is_some_and(|id| {
            let mut state = observer.accounting.lock().unwrap();
            match state.requests.get_mut(id) {
                Some(pending) if !pending.responding => {
                    pending.responding = true;
                    true
                }
                _ => false,
            }
        });
        async move {
            // This adapter advertises no server-initiated requests or change
            // notifications. Every outgoing message is a bounded response.
            let Some(id) = id else {
                observer.fail("unexpected unassociated response");
                return Err(invalid("response association"));
            };
            if !associated {
                observer.fail("unknown response identity");
                return Err(invalid("response association"));
            }
            output::write(writer, observer.clone(), item).await?;
            observer.accounting.lock().unwrap().requests.remove(&id);
            Ok(())
        }
    }
    async fn receive(&mut self) -> Option<ClientJsonRpcMessage> {
        loop {
            if let Some(pending) = self.protocol_error.as_mut() {
                let result = tokio::select! {
                    biased;
                    _ = self.observer.stopped.cancelled() => return None,
                    result = pending => result,
                };
                self.protocol_error = None;
                if result.is_err() {
                    return None;
                }
            }
            let line = tokio::select! {
                biased;
                _ = self.observer.stopped.cancelled() => return None,
                line = self.read.next() => match line {
                    Some(line) => line,
                    None => { self.observer.stopped.cancel(); return None; }
                },
            };
            let line = match line {
                Ok(line) => line,
                Err(_) => {
                    self.observer.fail("invalid or over-budget input frame");
                    return None;
                }
            };
            let mut message = match decode::decode(&line) {
                decode::Decoded::Message(message) => *message,
                decode::Decoded::Ignore => {
                    tokio::task::yield_now().await;
                    continue;
                }
                decode::Decoded::Fatal(message) => {
                    self.observer.fail(message);
                    return None;
                }
                decode::Decoded::Fault { id, error } => {
                    let aliases_active = id.as_ref().is_some_and(|id| {
                        let state = self.observer.accounting.lock().unwrap();
                        state.requests.contains_key(id) || state.retired.contains(id)
                    });
                    if aliases_active {
                        self.observer
                            .fail("invalid request aliases an active identity");
                        return None;
                    }
                    // One additional bounded response is reserved for protocol
                    // faults. No next frame is read until it has actually flushed.
                    self.protocol_error = Some(output::write(
                        self.write.clone(),
                        self.observer.clone(),
                        ServerJsonRpcMessage::error(error, id),
                    ));
                    tokio::task::yield_now().await;
                    continue;
                }
            };
            match &mut message {
                JsonRpcMessage::Request(request) => {
                    if self.observer.admit(request).is_err() {
                        self.observer.fail("request identity or in-flight budget");
                        return None;
                    }
                }
                JsonRpcMessage::Notification(n) => {
                    // Both supported stdio versions let the client cancel its
                    // own request. HTTP stream cancellation and server-originated
                    // subscription cancellation are different transport roles.
                    if let ClientNotification::CancelledNotification(c) = &n.notification
                        && let Some(id) = &c.params.request_id
                        && let Some(pending) = self
                            .observer
                            .accounting
                            .lock()
                            .unwrap()
                            .requests
                            .get_mut(id)
                    {
                        pending.cancelled = true;
                    }
                    let count = self.observer.notifications.fetch_add(1, Ordering::AcqRel);
                    if count >= NOTIFICATIONS {
                        self.observer.notifications.fetch_sub(1, Ordering::AcqRel);
                        self.observer.fail("notification in-flight budget");
                        return None;
                    }
                    n.notification
                        .extensions_mut()
                        .insert(Arc::new(NotificationLife(
                            self.observer.notifications.clone(),
                        )));
                }
                _ => {
                    self.observer.fail("unsolicited client response");
                    return None;
                }
            }
            return Some(message);
        }
    }
    async fn close(&mut self) -> io::Result<()> {
        self.observer.stopped.cancel();
        Ok(())
    }
}

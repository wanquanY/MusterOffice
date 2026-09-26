use super::*;
use serde_json::json;
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};

fn ping(id: u32) -> String {
    format!("{{\"jsonrpc\":\"2.0\",\"id\":{id},\"method\":\"ping\"}}\n")
}
fn cancel(id: u32) -> String {
    format!(
        "{{\"jsonrpc\":\"2.0\",\"method\":\"notifications/cancelled\",\"params\":{{\"requestId\":{id}}}}}\n"
    )
}
fn response(id: u32) -> ServerJsonRpcMessage {
    serde_json::from_value(json!({"jsonrpc":"2.0","id":id,"result":{}})).unwrap()
}
fn transport(
    input: String,
) -> (
    BoundedTransport<std::io::Cursor<Vec<u8>>, tokio::io::Sink>,
    Arc<Observer>,
) {
    BoundedTransport::new(std::io::Cursor::new(input.into_bytes()), tokio::io::sink())
}

#[tokio::test]
async fn over_budget_unterminated_and_unsolicited_frames_fail_closed() {
    for (index, input) in [
        "x".repeat(INPUT_BYTES + 1) + "\n",
        ping(1).trim_end().to_owned(),
        "{\"jsonrpc\":\"2.0\",\"id\":1,\"result\":{}}\n".to_owned(),
    ]
    .into_iter()
    .enumerate()
    {
        let (mut transport, observer) = transport(input);
        assert!(
            transport.receive().await.is_none(),
            "invalid case {index} was admitted"
        );
        assert!(observer.error().is_some());
    }
}

#[tokio::test]
async fn malformed_messages_recover_without_dispatching_or_admitting_work() {
    let cases: Vec<(Vec<u8>, i32, Option<serde_json::Value>)> = vec![
        (b"{bad}\n".to_vec(), -32700, None),
        (b"\xff\n".to_vec(), -32700, None),
        (
            b"{\"jsonrpc\":\"2.0\",\"id\":1,\"id\":2,\"method\":\"ping\"}\n".to_vec(),
            -32600,
            None,
        ),
        (ping(1).replace("1", "null").into_bytes(), -32600, None),
        (b"[]\n".to_vec(), -32600, None),
        (
            b"{\"jsonrpc\":\"1.0\",\"id\":2,\"method\":\"ping\"}\n".to_vec(),
            -32600,
            Some(json!(2)),
        ),
        (
            b"{\"jsonrpc\":\"2.0\",\"id\":3,\"method\":\"tools/call\",\"params\":{}}\n".to_vec(),
            -32602,
            Some(json!(3)),
        ),
        (
            b"{\"jsonrpc\":\"2.0\",\"id\":4,\"method\":\"resources/read\",\"params\":[]}\n"
                .to_vec(),
            -32602,
            Some(json!(4)),
        ),
    ];
    for (mut bytes, code, id) in cases {
        bytes.extend(ping(10).bytes());
        let (writer, reader) = tokio::io::duplex(8192);
        let (mut transport, observer) = BoundedTransport::new(std::io::Cursor::new(bytes), writer);
        let message = transport.receive().await.unwrap();
        assert!(
            matches!(message, JsonRpcMessage::Request(r) if r.id == serde_json::from_value(json!(10)).unwrap())
        );
        let mut line = String::new();
        BufReader::new(reader).read_line(&mut line).await.unwrap();
        let error: serde_json::Value = serde_json::from_str(&line).unwrap();
        assert_eq!(error["error"]["code"], code);
        assert_eq!(error.get("id"), id.as_ref());
        assert_eq!(observer.accounting.lock().unwrap().requests.len(), 1);
        assert!(observer.error().is_none());
    }
}

#[tokio::test]
async fn partial_protocol_error_survives_cancellation_of_receive() {
    let (writer, mut reader) = tokio::io::duplex(1);
    let (mut transport, observer) = BoundedTransport::new(
        std::io::Cursor::new(("{bad}\n".to_owned() + &ping(1)).into_bytes()),
        writer,
    );
    assert!(
        tokio::time::timeout(Duration::from_millis(10), transport.receive())
            .await
            .is_err()
    );
    assert!(transport.protocol_error.is_some());
    let mut first = [0];
    reader.read_exact(&mut first).await.unwrap();
    assert_eq!(first[0], b'{');
    let read = tokio::spawn(async move {
        let mut rest = String::from("{");
        BufReader::new(reader).read_line(&mut rest).await.unwrap();
        rest
    });
    assert!(transport.receive().await.is_some());
    let error: serde_json::Value = serde_json::from_str(&read.await.unwrap()).unwrap();
    assert_eq!(error["error"]["code"], -32700);
    assert!(transport.protocol_error.is_none() && observer.error().is_none());
}

#[tokio::test]
async fn invalid_notifications_never_get_responses_or_mutate_cancellation() {
    let bytes = b"{\"jsonrpc\":\"2.0\",\"method\":\"notifications/cancelled\",\"params\":7}\n\
        {\"jsonrpc\":\"2.0\",\"method\":\"notifications/unknown\",\"params\":{}}\n";
    let (mut transport, observer) =
        transport(String::from_utf8(bytes.to_vec()).unwrap() + &ping(1));
    assert!(matches!(
        transport.receive().await,
        Some(JsonRpcMessage::Request(_))
    ));
    assert!(transport.protocol_error.is_none());
    assert_eq!(observer.notifications.load(Ordering::Acquire), 0);
}

#[tokio::test]
async fn fragmented_frame_survives_cancelled_receive() {
    let (mut input, read) = tokio::io::duplex(1024);
    let (mut transport, observer) = BoundedTransport::new(read, tokio::io::sink());
    let data = ping(1);
    input.write_all(&data.as_bytes()[..10]).await.unwrap();
    assert!(
        tokio::time::timeout(Duration::from_millis(10), transport.receive())
            .await
            .is_err()
    );
    input.write_all(&data.as_bytes()[10..]).await.unwrap();
    assert!(transport.receive().await.is_some());
    assert!(observer.error().is_none());
}

#[tokio::test]
async fn malformed_request_cannot_alias_active_request_in_error_response() {
    let (mut transport, observer) = transport(
        ping(1) + "{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"tools/call\",\"params\":{}}\n",
    );
    drop(transport.receive().await.unwrap());
    assert!(transport.receive().await.is_none());
    assert_eq!(
        observer.error(),
        Some("invalid request aliases an active identity")
    );
    assert!(transport.protocol_error.is_none());
}

#[tokio::test]
async fn completed_handler_does_not_release_unwritten_response_budget() {
    let (mut transport, observer) = transport((1..=9).map(ping).collect());
    for _ in 0..REQUESTS {
        drop(transport.receive().await.unwrap());
    }
    let pending_send = transport.send(response(1));
    assert!(transport.receive().await.is_none());
    assert_eq!(observer.accounting.lock().unwrap().requests.len(), REQUESTS);
    drop(pending_send);
}

#[tokio::test]
async fn flushed_response_releases_admission() {
    let (mut transport, observer) = transport((1..=20).map(ping).collect());
    for id in 1..=20 {
        drop(transport.receive().await.unwrap());
        transport.send(response(id)).await.unwrap();
    }
    assert!(observer.accounting.lock().unwrap().requests.is_empty());
    assert!(observer.error().is_none());
}

#[tokio::test]
async fn cancellation_keeps_live_handler_and_retires_its_id_after_return() {
    let (mut transport, observer) =
        transport(ping(1) + &cancel(1) + &ping(2) + &ping(3) + &ping(1));
    let handler = transport.receive().await.unwrap();
    drop(transport.receive().await.unwrap()); // cancellation notification
    drop(transport.receive().await.unwrap()); // request 2
    let id = serde_json::from_value(json!(1)).unwrap();
    assert!(
        observer
            .accounting
            .lock()
            .unwrap()
            .requests
            .contains_key(&id)
    );
    drop(handler);
    drop(transport.receive().await.unwrap()); // request 3 collects handler 1
    assert!(observer.accounting.lock().unwrap().retired.contains(&id));
    assert!(transport.receive().await.is_none()); // cannot alias late SDK response
}

#[tokio::test]
async fn late_cancel_cannot_retire_response_before_its_write_future_is_polled() {
    let (mut transport, observer) = transport(ping(1) + &cancel(1) + &ping(2));
    drop(transport.receive().await.unwrap());
    let send = transport.send(response(1));
    drop(transport.receive().await.unwrap());
    drop(transport.receive().await.unwrap());
    assert_eq!(observer.accounting.lock().unwrap().requests.len(), 2);
    send.await.unwrap();
    transport.send(response(2)).await.unwrap();
    assert!(observer.error().is_none());
}

#[tokio::test]
async fn cancelled_requests_do_not_exhaust_the_active_request_budget() {
    let count = REQUESTS as u32 + 2;
    let input = (1..=count)
        .map(|id| ping(id) + &cancel(id))
        .collect::<String>()
        + &ping(999);
    let (mut transport, observer) = transport(input);
    for _ in 1..=count {
        drop(transport.receive().await.unwrap());
        drop(transport.receive().await.unwrap());
    }
    drop(transport.receive().await.unwrap());
    let state = observer.accounting.lock().unwrap();
    assert_eq!(state.retired.len(), count as usize);
    assert_eq!(state.requests.len(), 1);
    assert!(state.error.is_none());
}

#[tokio::test]
async fn notifications_have_an_independent_bounded_lifetime() {
    let notice = "{\"jsonrpc\":\"2.0\",\"method\":\"notifications/initialized\"}\n";
    let (mut transport, observer) = transport(notice.repeat(NOTIFICATIONS + 1));
    let mut handlers = Vec::new();
    for _ in 0..NOTIFICATIONS {
        handlers.push(transport.receive().await.unwrap());
    }
    assert!(transport.receive().await.is_none());
    drop(handlers);
    assert_eq!(observer.notifications.load(Ordering::Acquire), 0);
}

#[tokio::test]
async fn output_is_bounded_before_writing_any_frame() {
    let (mut transport, observer) = transport(ping(1));
    drop(transport.receive().await.unwrap());
    let response = serde_json::from_value(json!({"jsonrpc":"2.0","id":1,
        "error":{"code":-32603,"message":"x".repeat(OUTPUT_BYTES)}}))
    .unwrap();
    assert!(transport.send(response).await.is_err());
    assert_eq!(observer.error(), Some("response byte budget or encoding"));
}

#[tokio::test]
async fn slow_writer_retains_budget_until_flush() {
    let (write, mut read) = tokio::io::duplex(1);
    let (mut transport, observer) =
        BoundedTransport::new(std::io::Cursor::new(ping(1).into_bytes()), write);
    drop(transport.receive().await.unwrap());
    let send = tokio::spawn(transport.send(response(1)));
    tokio::task::yield_now().await;
    assert!(!send.is_finished());
    assert_eq!(observer.accounting.lock().unwrap().requests.len(), 1);
    let reader = tokio::spawn(async move {
        use tokio::io::AsyncReadExt;
        let mut byte = [0];
        let mut count = 0;
        loop {
            read.read_exact(&mut byte).await.unwrap();
            count += 1;
            if byte[0] == b'\n' {
                return count;
            }
        }
    });
    send.await.unwrap().unwrap();
    assert!(reader.await.unwrap() > 10);
    assert!(observer.accounting.lock().unwrap().requests.is_empty());
}

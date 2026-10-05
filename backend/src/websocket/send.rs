use tokio::sync::mpsc;

use dialect_coach_shared::ServerMessage;

/// Serialize a server message onto a connection's outgoing channel.
/// A closed channel means the connection is gone; the client reports the request as lost.
pub fn send(tx: &mpsc::UnboundedSender<String>, msg: &ServerMessage) {
    match serde_json::to_string(msg) {
        Ok(json) => {
            if tx.send(json).is_err() {
                tracing::info!("Connection closed before a server message could be sent");
            }
        }
        Err(e) => tracing::error!("Failed to serialize server message: {}", e),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dialect_coach_shared::Reply;
    use uuid::Uuid;

    #[test]
    fn test_send_serializes_server_message() {
        let (tx, mut rx) = mpsc::unbounded_channel();
        let id = Uuid::new_v4();

        send(
            &tx,
            &ServerMessage::Reply {
                id,
                body: Reply::Failed("internal error".to_string()),
            },
        );

        let parsed: ServerMessage = serde_json::from_str(&rx.try_recv().unwrap()).unwrap();
        match parsed {
            ServerMessage::Reply {
                id: got,
                body: Reply::Failed(e),
            } => assert_eq!((got, e.as_str()), (id, "internal error")),
            other => panic!("unexpected message: {:?}", other),
        }
    }
}

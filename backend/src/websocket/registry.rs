use std::collections::HashMap;

use dialect_coach_shared::{ServerMessage, UsageStats};
use tokio::sync::mpsc::UnboundedSender;
use uuid::Uuid;

use super::send;
use crate::state::Connections;

type Senders = HashMap<Uuid, Vec<UnboundedSender<String>>>;

/// Register a connection as signed in to `user_id`, moving it off any previous user.
/// A connection that closed while its sign-in was running stays unregistered.
pub fn bind(senders: &mut Senders, tx: &UnboundedSender<String>, user_id: Uuid) {
    release(senders, tx);
    if !tx.is_closed() {
        senders.entry(user_id).or_default().push(tx.clone());
    }
}

/// Remove a connection from whichever user it is registered to.
pub fn release(senders: &mut Senders, tx: &UnboundedSender<String>) {
    senders.retain(|_, txs| {
        txs.retain(|other| !other.same_channel(tx));
        !txs.is_empty()
    });
}

/// Send the user's current usage to every connection signed in to them.
pub async fn push_usage(connections: &Connections, user_id: Uuid, usage_stats: UsageStats) {
    let senders = connections.lock().await;
    let message = ServerMessage::UsageStats(usage_stats);
    for tx in senders.get(&user_id).into_iter().flatten() {
        send::send(tx, &message);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use tokio::sync::{Mutex, mpsc};

    fn channel() -> (UnboundedSender<String>, mpsc::UnboundedReceiver<String>) {
        mpsc::unbounded_channel()
    }

    fn count(senders: &Senders, user_id: Uuid) -> usize {
        senders.get(&user_id).map_or(0, Vec::len)
    }

    #[test]
    fn test_bind_registers_connection() {
        let (tx, _rx) = channel();
        let user = Uuid::new_v4();
        let mut senders = Senders::new();
        bind(&mut senders, &tx, user);
        assert_eq!(count(&senders, user), 1);
    }

    #[test]
    fn test_bind_skips_closed_connection() {
        let (tx, rx) = channel();
        drop(rx);
        let user = Uuid::new_v4();
        let mut senders = Senders::new();
        bind(&mut senders, &tx, user);
        assert!(!senders.contains_key(&user));
    }

    #[test]
    fn test_rebind_moves_connection_to_new_user() {
        let (tx, _rx) = channel();
        let (first, second) = (Uuid::new_v4(), Uuid::new_v4());
        let mut senders = Senders::new();
        bind(&mut senders, &tx, first);
        bind(&mut senders, &tx, second);
        assert_eq!((count(&senders, first), count(&senders, second)), (0, 1));
        assert!(!senders.contains_key(&first));
    }

    #[test]
    fn test_release_removes_only_its_own_connection() {
        let ((tab1, _rx1), (tab2, _rx2)) = (channel(), channel());
        let user = Uuid::new_v4();
        let mut senders = Senders::new();
        bind(&mut senders, &tab1, user);
        bind(&mut senders, &tab2, user);
        release(&mut senders, &tab1);
        assert_eq!(count(&senders, user), 1);
        assert!(senders[&user][0].same_channel(&tab2));
    }

    #[tokio::test]
    async fn test_push_usage_reaches_every_tab_of_the_user() {
        let ((tab1, mut rx1), (tab2, mut rx2)) = (channel(), channel());
        let user = Uuid::new_v4();
        let connections: Connections = Arc::new(Mutex::new(Senders::new()));
        bind(&mut *connections.lock().await, &tab1, user);
        bind(&mut *connections.lock().await, &tab2, user);

        push_usage(&connections, user, UsageStats::default()).await;

        for rx in [&mut rx1, &mut rx2] {
            let parsed: ServerMessage = serde_json::from_str(&rx.try_recv().unwrap()).unwrap();
            assert!(matches!(parsed, ServerMessage::UsageStats(_)));
        }
    }

    #[tokio::test]
    async fn test_push_usage_to_unbound_user_sends_nothing() {
        let (tab, mut rx) = channel();
        let connections: Connections = Arc::new(Mutex::new(Senders::new()));
        bind(&mut *connections.lock().await, &tab, Uuid::new_v4());

        push_usage(&connections, Uuid::new_v4(), UsageStats::default()).await;

        assert!(rx.try_recv().is_err());
    }
}

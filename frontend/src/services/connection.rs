//! The app's single WebSocket connection. Every request travels over it and
//! gets exactly one outcome: its reply, `Failed`, or `Lost` when the socket
//! dropped before the reply arrived. The socket reopens forever, one second
//! after each close.

use dialect_coach_shared::{
    ClientEnvelope, ClientMessage, Reply, RequestId, ServerMessage, UsageStats,
};
use futures_channel::oneshot;
use gloo_timers::callback::Timeout;
use log::{error, info};
use std::cell::RefCell;
use std::collections::HashMap;
use std::future::Future;
use std::rc::{Rc, Weak};
use uuid::Uuid;
use wasm_bindgen::JsCast;
use wasm_bindgen::closure::Closure;
use wasm_bindgen_futures::spawn_local;
use web_sys::{MessageEvent, WebSocket};
use yew::Callback;

const RETRY_DELAY_MS: u32 = 1000;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ConnectionStatus {
    #[default]
    Connecting,
    Connected,
    Reconnecting,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RequestError {
    /// The socket dropped after the request was sent and before its reply arrived.
    Lost,
    /// The backend failed while handling the request.
    Failed(String),
}

impl std::fmt::Display for RequestError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RequestError::Lost => write!(f, "Connection dropped before a reply arrived"),
            RequestError::Failed(e) => write!(f, "{}", e),
        }
    }
}

impl std::error::Error for RequestError {}

/// Which requests are queued, awaiting replies, or lost. Pure bookkeeping.
#[derive(Default)]
pub struct LinkState {
    status: ConnectionStatus,
    /// Non-empty only while not connected.
    outbox: Vec<ClientEnvelope>,
    /// Sent on the current socket, unanswered.
    in_flight: Vec<ClientEnvelope>,
    /// In flight when the socket dropped; reported when the next socket opens.
    lost: Vec<ClientEnvelope>,
}

pub struct Opened {
    pub lost: Vec<ClientEnvelope>,
    pub transmit: Vec<ClientEnvelope>,
}

impl LinkState {
    /// While connected, track the request and return it to transmit now; otherwise queue it.
    pub fn submit(&mut self, envelope: ClientEnvelope) -> Option<ClientEnvelope> {
        if self.status != ConnectionStatus::Connected {
            self.outbox.push(envelope);
            return None;
        }
        self.in_flight.push(envelope.clone());
        Some(envelope)
    }

    /// The socket opened: report what was lost and send what was queued.
    pub fn opened(&mut self) -> Opened {
        self.status = ConnectionStatus::Connected;
        let transmit = std::mem::take(&mut self.outbox);
        self.in_flight.extend(transmit.iter().cloned());
        Opened {
            lost: std::mem::take(&mut self.lost),
            transmit,
        }
    }

    pub fn answered(&mut self, id: RequestId) {
        self.in_flight.retain(|envelope| envelope.id != id);
    }

    /// The socket closed: whatever was awaiting a reply is lost.
    pub fn dropped(&mut self) {
        self.lost.append(&mut self.in_flight);
        self.status = ConnectionStatus::Reconnecting;
    }

    pub fn status(&self) -> ConnectionStatus {
        self.status
    }
}

#[derive(Clone, Default)]
pub struct ConnectionEvents {
    pub on_status: Callback<ConnectionStatus>,
    /// Emitted before queued requests are sent, so requests made here go first.
    pub on_open: Callback<()>,
    /// The requests lost while the connection was down, reported once it is back.
    pub on_lost: Callback<Vec<ClientMessage>>,
    pub on_push: Callback<UsageStats>,
}

type Waiter = oneshot::Sender<Result<Reply, RequestError>>;

struct Shell {
    url: String,
    events: ConnectionEvents,
    link: LinkState,
    waiters: HashMap<RequestId, Waiter>,
    socket: Option<Socket>,
    retry: Option<Timeout>,
}

/// An open or opening browser socket and the handlers attached to it.
struct Socket {
    ws: WebSocket,
    _on_open: Closure<dyn FnMut()>,
    _on_message: Closure<dyn FnMut(MessageEvent)>,
    _on_close: Closure<dyn FnMut()>,
}

impl Drop for Socket {
    fn drop(&mut self) {
        self.ws.set_onopen(None);
        self.ws.set_onmessage(None);
        self.ws.set_onclose(None);
        self.ws
            .close()
            .expect("close() without a code or reason cannot throw");
    }
}

#[derive(Clone)]
pub struct Connection(Rc<RefCell<Shell>>);

impl PartialEq for Connection {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}

impl Connection {
    pub fn new(url: &str) -> Self {
        Self(Rc::new(RefCell::new(Shell {
            url: url.to_string(),
            events: ConnectionEvents::default(),
            link: LinkState::default(),
            waiters: HashMap::new(),
            socket: None,
            retry: None,
        })))
    }

    pub fn set_events(&self, events: ConnectionEvents) {
        self.0.borrow_mut().events = events;
    }

    pub fn start(&self) {
        open_socket(&self.0);
    }

    /// Submit now (sent at once while connected, otherwise when the socket opens)
    /// and resolve with the request's single outcome.
    pub fn request(
        &self,
        body: ClientMessage,
    ) -> impl Future<Output = Result<Reply, RequestError>> + use<> {
        let id = Uuid::new_v4();
        let outcome = self.submit(ClientEnvelope { id, body });
        async move {
            outcome
                .await
                .expect("every request is resolved exactly once")
        }
    }

    fn submit(&self, envelope: ClientEnvelope) -> oneshot::Receiver<Result<Reply, RequestError>> {
        let (waiter, outcome) = oneshot::channel();
        let transmit_now = {
            let mut shell = self.0.borrow_mut();
            shell.waiters.insert(envelope.id, waiter);
            shell.link.submit(envelope)
        };
        if let Some(envelope) = transmit_now {
            transmit(&self.0, &envelope);
        }
        outcome
    }
}

fn shell_of(weak: &Weak<RefCell<Shell>>) -> Rc<RefCell<Shell>> {
    weak.upgrade()
        .expect("a socket's handlers are detached before its connection is dropped")
}

fn open_socket(shell: &Rc<RefCell<Shell>>) {
    let url = shell.borrow().url.clone();
    info!("Opening WebSocket to {}", url);
    let ws = WebSocket::new(&url).expect("the WebSocket URL is valid");
    let socket = attach(ws, Rc::downgrade(shell));
    shell.borrow_mut().socket = Some(socket);
}

fn attach(ws: WebSocket, shell: Weak<RefCell<Shell>>) -> Socket {
    let weak = shell.clone();
    let on_open = Closure::<dyn FnMut()>::new(move || handle_open(&shell_of(&weak)));
    let weak = shell.clone();
    let on_message =
        Closure::<dyn FnMut(MessageEvent)>::new(move |e| handle_message(&shell_of(&weak), e));
    let on_close = Closure::<dyn FnMut()>::new(move || handle_close(&shell_of(&shell)));
    ws.set_onopen(Some(on_open.as_ref().unchecked_ref()));
    ws.set_onmessage(Some(on_message.as_ref().unchecked_ref()));
    ws.set_onclose(Some(on_close.as_ref().unchecked_ref()));
    Socket {
        ws,
        _on_open: on_open,
        _on_message: on_message,
        _on_close: on_close,
    }
}

fn transmit(shell: &Rc<RefCell<Shell>>, envelope: &ClientEnvelope) {
    let json = serde_json::to_string(envelope).expect("protocol messages serialize");
    let shell = shell.borrow();
    let socket = shell
        .socket
        .as_ref()
        .expect("connected means a socket exists");
    socket
        .ws
        .send_with_str(&json)
        .expect("send on an opened socket cannot throw");
}

fn handle_open(shell: &Rc<RefCell<Shell>>) {
    info!("WebSocket connected");
    let (opened, events) = {
        let mut shell = shell.borrow_mut();
        (shell.link.opened(), shell.events.clone())
    };
    events.on_status.emit(ConnectionStatus::Connected);
    events.on_open.emit(());
    report_lost(shell, opened.lost, &events);
    for envelope in &opened.transmit {
        transmit(shell, envelope);
    }
}

fn report_lost(shell: &Rc<RefCell<Shell>>, lost: Vec<ClientEnvelope>, events: &ConnectionEvents) {
    if lost.is_empty() {
        return;
    }
    let waiters: Vec<Waiter> = {
        let mut shell = shell.borrow_mut();
        lost.iter()
            .filter_map(|envelope| shell.waiters.remove(&envelope.id))
            .collect()
    };
    for waiter in waiters {
        resolve(waiter, Err(RequestError::Lost));
    }
    events
        .on_lost
        .emit(lost.into_iter().map(|envelope| envelope.body).collect());
}

/// A caller that stopped waiting has nothing left to tell.
fn resolve(waiter: Waiter, outcome: Result<Reply, RequestError>) {
    let _ = waiter.send(outcome);
}

fn handle_message(shell: &Rc<RefCell<Shell>>, event: MessageEvent) {
    let text = event
        .data()
        .as_string()
        .expect("the server sends text frames");
    match serde_json::from_str::<ServerMessage>(&text) {
        Ok(ServerMessage::Reply { id, body }) => handle_reply(shell, id, body),
        Ok(ServerMessage::UsageStats(stats)) => {
            let on_push = shell.borrow().events.on_push.clone();
            on_push.emit(stats);
        }
        Err(e) => error!("Unparseable server message: {}", e),
    }
}

fn handle_reply(shell: &Rc<RefCell<Shell>>, id: RequestId, body: Reply) {
    let waiter = {
        let mut shell = shell.borrow_mut();
        shell.link.answered(id);
        shell.waiters.remove(&id)
    };
    let outcome = match body {
        Reply::Failed(e) => Err(RequestError::Failed(e)),
        reply => Ok(reply),
    };
    match waiter {
        Some(waiter) => resolve(waiter, outcome),
        None => error!("Reply for unknown request {}", id),
    }
}

fn handle_close(shell: &Rc<RefCell<Shell>>) {
    info!("WebSocket closed; reopening in {}ms", RETRY_DELAY_MS);
    let (socket, on_status) = {
        let mut shell = shell.borrow_mut();
        shell.link.dropped();
        (shell.socket.take(), shell.events.on_status.clone())
    };
    // This handler belongs to the socket; free it after the handler returns.
    spawn_local(async move { drop(socket) });
    on_status.emit(ConnectionStatus::Reconnecting);
    let weak = Rc::downgrade(shell);
    let retry = Timeout::new(RETRY_DELAY_MS, move || open_socket(&shell_of(&weak)));
    shell.borrow_mut().retry = Some(retry);
}

#[cfg(test)]
mod tests {
    use super::*;
    use dialect_coach_shared::AccountRequest;

    #[test]
    fn test_request_error_message() {
        assert_eq!(
            RequestError::Lost.to_string(),
            "Connection dropped before a reply arrived"
        );
        assert_eq!(
            RequestError::Failed("internal error".to_string()).to_string(),
            "internal error"
        );
    }

    fn envelope(token: &str) -> ClientEnvelope {
        ClientEnvelope {
            id: Uuid::new_v4(),
            body: ClientMessage::Account(AccountRequest::ValidateSession {
                token: token.to_string(),
            }),
        }
    }

    fn ids(envelopes: &[ClientEnvelope]) -> Vec<RequestId> {
        envelopes.iter().map(|envelope| envelope.id).collect()
    }

    fn connected() -> LinkState {
        let mut link = LinkState::default();
        link.opened();
        link
    }

    #[test]
    fn test_submit_while_connecting_queues() {
        let mut link = LinkState::default();
        assert!(link.submit(envelope("a")).is_none());
        assert_eq!(link.status(), ConnectionStatus::Connecting);
    }

    #[test]
    fn test_submit_while_connected_transmits() {
        let mut link = connected();
        let sent = envelope("a");
        assert_eq!(link.submit(sent.clone()).map(|e| e.id), Some(sent.id));
    }

    #[test]
    fn test_opened_drains_queue_in_order_into_flight() {
        let mut link = LinkState::default();
        let (first, second) = (envelope("a"), envelope("b"));
        link.submit(first.clone());
        link.submit(second.clone());

        let opened = link.opened();

        assert_eq!(ids(&opened.transmit), vec![first.id, second.id]);
        assert_eq!(link.status(), ConnectionStatus::Connected);
        link.dropped();
        assert_eq!(ids(&link.opened().lost), vec![first.id, second.id]);
    }

    #[test]
    fn test_drop_while_connected_loses_unanswered_requests() {
        let mut link = connected();
        let (answered, unanswered) = (envelope("a"), envelope("b"));
        link.submit(answered.clone());
        link.submit(unanswered.clone());
        link.answered(answered.id);

        link.dropped();

        assert_eq!(link.status(), ConnectionStatus::Reconnecting);
        assert_eq!(ids(&link.opened().lost), vec![unanswered.id]);
    }

    #[test]
    fn test_drop_while_not_connected_loses_nothing() {
        let mut link = LinkState::default();
        let queued = envelope("a");
        link.submit(queued.clone());

        link.dropped();
        let opened = link.opened();

        assert!(opened.lost.is_empty());
        assert_eq!(ids(&opened.transmit), vec![queued.id]);
    }

    #[test]
    fn test_lost_is_reported_once() {
        let mut link = connected();
        link.submit(envelope("a"));
        link.dropped();

        assert_eq!(link.opened().lost.len(), 1);
        link.dropped();
        assert!(link.opened().lost.is_empty());
    }
}

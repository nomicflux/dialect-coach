use dialect_coach_shared::{AuthCredentials, User, UserMessage};
use futures_util::{SinkExt, StreamExt};
use gloo_net::websocket::{Message as WsMessage, futures::WebSocket};
use log::{error, info};
use std::cell::RefCell;
use std::rc::Rc;
use wasm_bindgen_futures::spawn_local;
use yew::Callback;

/// WebSocket service for user management
pub struct UserWebSocketService {
    sender: Rc<RefCell<Option<futures_channel::mpsc::UnboundedSender<String>>>>,
    url: String,
    on_create_response: Callback<Result<(User, String), String>>,
    on_signin_response: Callback<Result<(User, String), String>>,
    on_validate_session_response: Callback<Result<User, String>>,
    on_open: Callback<()>,
}

impl UserWebSocketService {
    /// Create a new user WebSocket service
    pub fn new(url: &str) -> Self {
        Self {
            sender: Rc::new(RefCell::new(None)),
            url: url.to_string(),
            on_create_response: Callback::noop(),
            on_signin_response: Callback::noop(),
            on_validate_session_response: Callback::noop(),
            on_open: Callback::noop(),
        }
    }

    /// Set callback for create user responses
    pub fn set_on_create_response(&mut self, callback: Callback<Result<(User, String), String>>) {
        self.on_create_response = callback;
    }

    /// Set callback for sign in responses
    pub fn set_on_signin_response(&mut self, callback: Callback<Result<(User, String), String>>) {
        self.on_signin_response = callback;
    }

    /// Set callback for validate session responses
    pub fn set_on_validate_session_response(&mut self, callback: Callback<Result<User, String>>) {
        self.on_validate_session_response = callback;
    }

    /// Set callback for connection open
    pub fn set_on_open(&mut self, callback: Callback<()>) {
        self.on_open = callback;
    }

    /// Connect to the WebSocket server
    pub fn connect(&mut self) {
        info!("Connecting to user WebSocket at: {}", self.url);

        let ws = match WebSocket::open(&self.url) {
            Ok(ws) => ws,
            Err(e) => {
                error!("Failed to open user WebSocket: {:?}", e);
                return;
            }
        };

        let (mut write, mut read) = ws.split();
        let (tx, mut rx) = futures_channel::mpsc::unbounded::<String>();
        *self.sender.borrow_mut() = Some(tx);

        let on_create = self.on_create_response.clone();
        let on_signin = self.on_signin_response.clone();
        let on_validate = self.on_validate_session_response.clone();
        let on_open = self.on_open.clone();

        // Spawn send task
        spawn_local(async move {
            while let Some(text) = rx.next().await {
                if let Err(e) = write.send(WsMessage::Text(text)).await {
                    error!("Failed to send user message: {:?}", e);
                    break;
                }
            }
        });

        // Spawn receive task
        spawn_local(async move {
            info!("User WebSocket connection established");
            on_open.emit(());

            while let Some(msg) = read.next().await {
                if let Ok(WsMessage::Text(text)) = msg {
                    process_message(&text, &on_create, &on_signin, &on_validate);
                }
            }

            info!("User WebSocket connection closed");
        });
    }

    /// Create a new user
    pub fn create_user(
        &self,
        username: String,
        email: String,
        credentials: AuthCredentials,
        password: String,
    ) -> Result<(), String> {
        let msg = UserMessage::CreateUser {
            username,
            email,
            credentials,
            password,
        };
        self.send_message(&msg)
    }

    /// Sign in with username and password
    pub fn sign_in(&self, username: String, password: String) -> Result<(), String> {
        let msg = UserMessage::SignIn { username, password };
        self.send_message(&msg)
    }

    /// Validate session with JWT token
    pub fn validate_session(&self, token: String) -> Result<(), String> {
        let msg = UserMessage::ValidateSession { token };
        self.send_message(&msg)
    }

    /// Send a UserMessage
    fn send_message(&self, msg: &UserMessage) -> Result<(), String> {
        let json = serde_json::to_string(msg).map_err(|e| format!("Failed to serialize: {}", e))?;

        if let Some(sender) = self.sender.borrow().as_ref() {
            sender
                .unbounded_send(json)
                .map_err(|e| format!("Failed to send: {}", e))?;
            Ok(())
        } else {
            Err("WebSocket not connected".to_string())
        }
    }
}

/// Process incoming UserMessage
fn process_message(
    text: &str,
    on_create: &Callback<Result<(User, String), String>>,
    on_signin: &Callback<Result<(User, String), String>>,
    on_validate: &Callback<Result<User, String>>,
) {
    match serde_json::from_str::<UserMessage>(text) {
        Ok(UserMessage::CreateUserResponse(result)) => {
            info!("Received CreateUserResponse: {:?}", result.is_ok());
            on_create.emit(result);
        }
        Ok(UserMessage::SignInResponse(result)) => {
            info!("Received SignInResponse: {:?}", result.is_ok());
            on_signin.emit(result);
        }
        Ok(UserMessage::ValidateSessionResponse(result)) => {
            info!("Received ValidateSessionResponse: {:?}", result.is_ok());
            on_validate.emit(result);
        }
        Ok(_) => {
            error!("Received unexpected UserMessage variant");
        }
        Err(e) => {
            error!("Failed to parse UserMessage: {}", e);
        }
    }
}

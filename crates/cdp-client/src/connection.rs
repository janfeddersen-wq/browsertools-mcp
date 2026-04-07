//! WebSocket connection management for Chrome DevTools Protocol.
//!
//! Handles the raw WebSocket transport, message framing, and
//! multiplexing of CDP sessions over a single connection.

use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicI64, Ordering};

use futures_util::stream::SplitSink;
use futures_util::{SinkExt, StreamExt};
use tokio::net::TcpStream;
use tokio::sync::{Mutex, mpsc, oneshot};
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream};

use crate::error::{CdpError, CdpResult};

type WsStream = WebSocketStream<MaybeTlsStream<TcpStream>>;
type WsSink = SplitSink<WsStream, Message>;

/// A pending CDP command awaiting its response.
struct PendingCommand {
    tx: oneshot::Sender<serde_json::Value>,
}

/// Manages event subscriptions for a specific CDP session.
type EventSender = mpsc::UnboundedSender<serde_json::Value>;

/// The raw WebSocket connection to a Chrome instance.
///
/// Handles message routing: command responses go to their pending oneshot channel,
/// events go to registered session event listeners.
pub struct CdpConnection {
    next_id: AtomicI64,
    writer: Arc<Mutex<WsSink>>,
    pending: Arc<Mutex<HashMap<i64, PendingCommand>>>,
    event_listeners: Arc<Mutex<HashMap<String, Vec<EventSender>>>>,
}

impl CdpConnection {
    /// Connect to a Chrome DevTools Protocol WebSocket endpoint.
    pub async fn connect(ws_url: &str) -> CdpResult<Self> {
        tracing::info!(url = ws_url, "Connecting to Chrome DevTools");

        let (ws_stream, _) = tokio_tungstenite::connect_async(ws_url).await?;

        let (writer, mut reader) = ws_stream.split();

        let pending: Arc<Mutex<HashMap<i64, PendingCommand>>> =
            Arc::new(Mutex::new(HashMap::new()));
        let event_listeners: Arc<Mutex<HashMap<String, Vec<EventSender>>>> =
            Arc::new(Mutex::new(HashMap::new()));

        let pending_clone = pending.clone();
        let events_clone = event_listeners.clone();

        // Spawn the reader task that routes incoming messages.
        tokio::spawn(async move {
            while let Some(msg) = reader.next().await {
                match msg {
                    Ok(Message::Text(ref text)) => {
                        if let Ok(json) = serde_json::from_str::<serde_json::Value>(text) {
                            Self::handle_message(json, &pending_clone, &events_clone).await;
                        }
                    }
                    Ok(Message::Close(_)) => {
                        tracing::info!("WebSocket closed by remote");
                        break;
                    }
                    Err(e) => {
                        tracing::error!(error = %e, "WebSocket read error");
                        break;
                    }
                    _ => {}
                }
            }
        });

        Ok(Self {
            next_id: AtomicI64::new(1),
            writer: Arc::new(Mutex::new(writer)),
            pending,
            event_listeners,
        })
    }

    async fn handle_message(
        json: serde_json::Value,
        pending: &Mutex<HashMap<i64, PendingCommand>>,
        event_listeners: &Mutex<HashMap<String, Vec<EventSender>>>,
    ) {
        // Check if this is a response to a command (has "id" field).
        if let Some(id) = json.get("id").and_then(|v| v.as_i64()) {
            let mut pending = pending.lock().await;
            if let Some(cmd) = pending.remove(&id) {
                let _ = cmd.tx.send(json);
            }
        }
        // Check if this is an event (has "method" field but no "id").
        else if let Some(method) = json.get("method").and_then(|v| v.as_str()) {
            let listeners = event_listeners.lock().await;
            if let Some(senders) = listeners.get(method) {
                for sender in senders {
                    let _ = sender.send(json.clone());
                }
            }
        }
    }

    /// Send a CDP command and wait for its response.
    pub async fn send_command(
        &self,
        method: &str,
        params: serde_json::Value,
        session_id: Option<&str>,
    ) -> CdpResult<serde_json::Value> {
        let id = self.next_id.fetch_add(1, Ordering::SeqCst);

        let mut msg = serde_json::json!({
            "id": id,
            "method": method,
            "params": params,
        });

        if let Some(sid) = session_id {
            msg["sessionId"] = serde_json::Value::String(sid.to_string());
        }

        let (tx, rx) = oneshot::channel();
        {
            let mut pending = self.pending.lock().await;
            pending.insert(id, PendingCommand { tx });
        }

        let text = serde_json::to_string(&msg)?;
        {
            let mut writer = self.writer.lock().await;
            writer
                .send(Message::Text(text.into()))
                .await
                .map_err(|e| CdpError::SendFailed(e.to_string()))?;
        }

        let response = rx.await.map_err(|_| CdpError::ConnectionClosed)?;

        // Check for CDP protocol errors.
        if let Some(error) = response.get("error") {
            let code = error.get("code").and_then(|v| v.as_i64()).unwrap_or(-1);
            let message = error
                .get("message")
                .and_then(|v| v.as_str())
                .unwrap_or("Unknown error")
                .to_string();
            return Err(CdpError::ProtocolError { code, message });
        }

        Ok(response
            .get("result")
            .cloned()
            .unwrap_or(serde_json::Value::Null))
    }

    /// Subscribe to CDP events by method name. Returns a receiver for events.
    pub async fn subscribe_events(
        &self,
        method: &str,
    ) -> mpsc::UnboundedReceiver<serde_json::Value> {
        let (tx, rx) = mpsc::unbounded_channel();
        let mut listeners = self.event_listeners.lock().await;
        listeners.entry(method.to_string()).or_default().push(tx);
        rx
    }

    /// Close the WebSocket connection.
    pub async fn close(&self) -> CdpResult<()> {
        let mut writer = self.writer.lock().await;
        writer
            .close()
            .await
            .map_err(|e| CdpError::SendFailed(e.to_string()))?;
        Ok(())
    }
}

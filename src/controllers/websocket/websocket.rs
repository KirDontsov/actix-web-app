use actix_web::{web, HttpRequest, Responder};
use actix_ws::{handle, Message};
use futures::StreamExt;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};
use uuid;

// Define a struct to hold WebSocket connections
#[derive(Clone)]
pub struct WebSocketConnections {
    connections: Arc<RwLock<HashMap<String, mpsc::UnboundedSender<String>>>>,
}

impl WebSocketConnections {
    pub fn new() -> Self {
        Self {
            connections: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn add_connection(&self, id: String, sender: mpsc::UnboundedSender<String>) {
        let mut connections = self.connections.write().await;
        connections.insert(id, sender);
    }

    pub async fn remove_connection(&self, id: &str) {
        let mut connections = self.connections.write().await;
        connections.remove(id);
    }

    pub async fn broadcast_message(&self, message: &str) {
        let connections = self.connections.read().await;
        for (_, sender) in connections.iter() {
            let _ = sender.send(message.to_string());
        }
    }
}

// Function to broadcast messages to all WebSocket connections
pub async fn broadcast_to_websockets(connections: &WebSocketConnections, message: &str) {
    connections.broadcast_message(message).await;
}

// WebSocket handler function
pub async fn websocket_handler(
    req: HttpRequest,
    body: web::Payload,
    connections: web::Data<WebSocketConnections>,
) -> actix_web::Result<impl Responder> {
    // Generate a unique ID for this connection
    let id = uuid::Uuid::new_v4().to_string();

    // Create a channel for sending messages to this connection
    let (tx, mut rx) = mpsc::unbounded_channel::<String>();

    // Add the connection to the global connections map
    connections.add_connection(id.clone(), tx).await;

    // Create the WebSocket context
    let (response, mut session, mut msg_stream) = handle(&req, body)?;

    // Clone connections for use in the spawned task
    let connections_clone = connections.clone();
    let id_clone = id.clone();

    // Process messages in a spawned task
    actix_web::rt::spawn(async move {
        loop {
            tokio::select! {
				// Handle incoming messages from the WebSocket
				msg_result = msg_stream.next() => {
					match msg_result {
						Some(Ok(Message::Ping(bytes))) => {
							if session.pong(&bytes).await.is_err() {
								break;
							}
						}
						Some(Ok(Message::Text(msg))) => {
							println!("Got text: {msg}");
						}
						Some(Ok(_)) => {
							// Other message types - continue processing
							continue;
						}
						Some(Err(_)) => {
							// Connection error - break the loop
							break;
						}
						None => {
							// Connection closed - break the loop
							break;
						}
					}
				}
				// Handle outgoing messages to the WebSocket
				msg = rx.recv() => {
					match msg {
						Some(text) => {
							if session.text(text).await.is_err() {
								break;
							}
						}
						None => {
							// Channel closed - break the loop
							break;
						}
					}
				}
			}
        }

        // Clean up the connection when the loop exits
        connections_clone.remove_connection(&id_clone).await;
        let _ = session.close(None).await;
    });

    Ok(response)
}

use crate::controllers::websocket::WebSocketConnections;
use futures::StreamExt;
use lapin::{
	options::{BasicConsumeOptions, QueueDeclareOptions},
	types::FieldTable,
	Channel,
};
use serde_json::Value;
use std::sync::Arc;

pub struct RabbitMQConsumer;

impl RabbitMQConsumer {
	pub async fn start_consumer(
		rabbitmq_channel: Channel,
		websocket_connections: Arc<WebSocketConnections>,
	) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
		// Declare the queue (crawler_progress)
		let queue_name = "crawler_progress";
		let queue = rabbitmq_channel
			.queue_declare(
				queue_name,
				QueueDeclareOptions::default(),
				FieldTable::default(),
			)
			.await?;

		println!("Declared queue: {}", queue.name());

		// Start consuming messages
		let consumer = rabbitmq_channel
			.basic_consume(
				queue.name().as_str(),
				"crawler_progress_consumer",
				BasicConsumeOptions::default(),
				FieldTable::default(),
			)
			.await?;

		println!("Started consuming from queue: {}", queue.name());

		// Process messages
		let websocket_connections_clone = websocket_connections.clone();
		let mut consumer_stream = consumer;

		while let Some(delivery_result) = consumer_stream.next().await {
			match delivery_result {
				Ok(delivery) => {
					// Process the message
					let message_data = String::from_utf8_lossy(&delivery.data).to_string();
					println!("Received message: {}", message_data);

					// Parse the message as JSON to validate it
					match serde_json::from_str::<Value>(&message_data) {
						Ok(json_value) => {
							// Send the message to all WebSocket connections
							let msg_str = json_value.to_string();
							let connections = websocket_connections_clone.clone();

							tokio::spawn(async move {
								connections.broadcast_message(&msg_str).await;
							});
						}
						Err(e) => {
							eprintln!("Failed to parse message as JSON: {}", e);
							// Send as string if JSON parsing fails
							let connections = websocket_connections_clone.clone();
							tokio::spawn(async move {
								connections.broadcast_message(&message_data).await;
							});
						}
					}

					// Acknowledge the message
					delivery
						.ack(lapin::options::BasicAckOptions::default())
						.await?;
				}
				Err(e) => {
					eprintln!("Error receiving message: {}", e);
				}
			}
		}

		Ok(())
	}
}

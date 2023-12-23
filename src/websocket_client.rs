use tokio_tungstenite::{connect_async, tungstenite::protocol::Message, WebSocketStream};
use futures_util::{StreamExt, SinkExt};
use url::Url;
use tokio::sync::{mpsc::Sender, Notify};
use std::sync::Arc;
use log::{info, error};
use serde::de::DeserializeOwned;
use std::any::type_name;

pub struct WebSocketClient<MessageType> {
    url: Url,
    sender: Sender<MessageType>,
    shutdown_notify: Arc<Notify>,
}

impl<MessageType> WebSocketClient<MessageType> 
where
    MessageType: DeserializeOwned + Send + 'static, // MessageType can be deserialized and sent across threads
{
    pub fn new(url: Url, sender: Sender<MessageType>, shutdown_notify: Arc<Notify>) -> Self {
        Self { url, sender, shutdown_notify }
    }

    pub async fn connect(&mut self) {
        let ws_stream = match connect_async(&self.url).await {
            Ok((stream, _)) => {
                info!("WebSocket handshake has been successfully completed with endpoint {} for {}<{}>.", self.url.to_string(), module_path!(), type_name::<MessageType>());
                stream
            }
            Err(e) => {
                error!("Failed to connect for {}<{}>: {:?}.", module_path!(), type_name::<MessageType>(), e);
                return;
            }
        };

        tokio::select! {
            _ = self.handle_messages(ws_stream) => {},
        }

    }

    async fn handle_messages(&mut self, ws_stream: WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>) {
        let (mut write, mut read) = ws_stream.split();

        loop {
            tokio::select! {
                _ = self.shutdown_notify.notified() => {
                    info!("Shutting down WebSocket connection for {}<{}>.", module_path!(), type_name::<MessageType>());
                    break;
                }
                message = read.next() => {
                    match message {
                        Some(Ok(Message::Text(text))) => {
                            if let Err(e) = self.process_message(&text) {
                                error!("Error processing message for {}<{}>: {:?}.", module_path!(), type_name::<MessageType>(), e);
                                // Here you can decide if you want to continue or take any specific action
                            }
                        },
                        Some(Ok(_)) => {}, // handle other message types if needed
                        Some(Err(e)) => {
                            error!("Error during the WebSocket communication for {}<{}>: {:?}.", module_path!(), type_name::<MessageType>(), e);
                            // Decide if you want to break or continue the loop
                        }
                        None => break, // Stream has ended or connection closed
                    }
                }
            }
        }

         // Initiate a graceful shutdown sequence
        info!("Sending close message to websocket server for {}<{}>.", module_path!(), type_name::<MessageType>());
        if let Err(e) = write.close().await {
            error!("Error sending close message for {}<{}>: {:?}.", module_path!(), type_name::<MessageType>(), e);
            return;
        }
        info!("Sent close message to websocket server successfully for {}<{}>.", module_path!(), type_name::<MessageType>());
    }

    fn process_message(&mut self, text: &str) -> Result<(), serde_json::Error> {
        let update = serde_json::from_str::<MessageType>(text)?; // Deserialize into type MessageType
        if let Err(e) = self.sender.try_send(update) {
            error!("Failed to send update for {}<{}>: {:?}.", module_path!(), type_name::<MessageType>(), e);
        }
        Ok(())
    }

}

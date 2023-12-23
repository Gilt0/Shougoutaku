use reqwest::{Client, Response};
use std::sync::Arc;
use tokio::sync::{mpsc::Sender, Notify};
use tokio::time::Duration;
use log::{info, error};
use serde::de::DeserializeOwned; // For JSON deserialization
use std::any::type_name;

pub struct HttpClient<ResponseType> {
    endpoint: String,
    sender: Sender<ResponseType>,
    fetch_interval: Duration,
    shutdown_notify: Arc<Notify>,
}

impl<ResponseType> HttpClient<ResponseType> 
where
    ResponseType: DeserializeOwned + Send + 'static, // Ensure ResponseType can be deserialized and sent across threads
{

    pub fn new(endpoint: String, sender: Sender<ResponseType>, fetch_interval_u64: u64, shutdown_notify: Arc<Notify>) -> Self {
        Self {
            endpoint,
            sender,
            fetch_interval: Duration::from_secs(fetch_interval_u64),
            shutdown_notify,
        }
    }

    pub async fn query_api(&mut self) {
        info!("Connecting to endpoint {} for {}<{}>.", self.endpoint, module_path!(), type_name::<ResponseType>());
        let client = Client::new();
        loop {
            // First, make a request
            match client.get(&self.endpoint).send().await {
                Ok(response) => {
                    if let Err(e) = self.process_response(response).await {
                        error!("Error processing response: {:?}", e);
                    }
                }
                Err(e) => {
                    error!("Error querying API: {:?}", e);
                },
            }
            // Then wait for the fetch interval or for the shutdown signal
            tokio::select! {
                _ = tokio::time::sleep(self.fetch_interval) => {},
                _ = self.shutdown_notify.notified() => {
                    info!("HTTP client shutdown initiated for {}<{}>.", module_path!(), type_name::<ResponseType>());
                    break;
                }
            }
        }
    }
        
    async fn process_response(&mut self, response: Response) -> Result<(), Box<dyn std::error::Error>> {
        let json_text = response.text().await?;
        let update: ResponseType = serde_json::from_str(&json_text)?;
        self.sender.try_send(update).map_err(|e| e.into())
    }

}

use futures_util::{
    stream::{SplitSink, SplitStream},
    SinkExt, StreamExt,
};
use serde::Serialize;
use tokio::net::TcpStream;
use tokio_tungstenite::{
    connect_async, tungstenite::client::IntoClientRequest, tungstenite::protocol::Message,
    MaybeTlsStream, WebSocketStream,
};

const WEBSOCKET_URL: &str = "wss://rpc.api-pump.fun/ws";

pub struct Subscriber {
    pub subsciptions: Vec<Subscription>,
    pub connection: Option<(
        SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, Message>,
        SplitStream<WebSocketStream<MaybeTlsStream<TcpStream>>>,
    )>,
}

#[derive(Serialize)]
pub struct Subscription {
    method: String,
    params: Vec<String>,
}

impl Subscriber {
    /// Connects to the WebSocket and returns the split read and write streams.
    pub async fn connect(&mut self) -> &mut Self {
        let url = WEBSOCKET_URL
            .into_client_request()
            .expect("Invalid WebSocket URL");
        let (ws_stream, _) = connect_async(url).await.expect("Failed to connect");

        println!("WebSocket connected to {}", WEBSOCKET_URL);
        let (write, read) = ws_stream.split();

        self.connection = Some((write, read));
        self
    }

    /// Subscribes to trades on pump.fun
    pub async fn subscribe_trades(&mut self) {
        let payload = Subscription {
            method: "subscribeTrades".to_string(),
            params: vec![],
        };

        if let Some((write, _)) = &mut self.connection {
            write
                .send(Message::Text(serde_json::to_string(&payload).unwrap()))
                .await
                .unwrap();
        }
        self.subsciptions.push(payload)
    }

    /// Unsubscribes from trades on pump.fun
    pub async fn unsubscribe_trades(&mut self) {
        let payload = Subscription {
            method: "unsubscribeTrades".to_string(),
            params: vec![],
        };

        if let Some((write, _)) = &mut self.connection {
            write
                .send(Message::Text(serde_json::to_string(&payload).unwrap()))
                .await
                .unwrap();
        }
        self.subsciptions.retain(|sub| sub.method != payload.method);
    }

    /// Subscribes to new pool creations
    pub async fn subscribe_new_pools(&mut self) {
        let payload = Subscription {
            method: "subscribeNewPools".to_string(),
            params: vec![],
        };

        if let Some((write, _)) = &mut self.connection {
            write
                .send(Message::Text(serde_json::to_string(&payload).unwrap()))
                .await
                .unwrap();
        }
        self.subsciptions.push(payload)
    }

    /// Unsubscribes from new pool creations
    pub async fn unsubscribe_new_pools(&mut self) {
        let payload = Subscription {
            method: "unsubscribeNewPools".to_string(),
            params: vec![],
        };
        if let Some((write, _)) = &mut self.connection {
            write
                .send(Message::Text(serde_json::to_string(&payload).unwrap()))
                .await
                .unwrap();
        }
        self.subsciptions.retain(|sub| sub.method != payload.method);
    }
    /// Subscribes to trades for a specific token
    pub async fn subscribe_token(&mut self, token: &str) {
        let payload = Subscription {
            method: "subscribeToken".to_string(),
            params: vec![token.to_string()],
        };
        if let Some((write, _)) = &mut self.connection {
            write
                .send(Message::Text(serde_json::to_string(&payload).unwrap()))
                .await
                .unwrap();
        }
        self.subsciptions.push(payload)
    }

    /// Unsubscribes from trades for a specific token
    pub async fn unsubscribe_token(&mut self, token: &str) {
        let payload = Subscription {
            method: "unsubscribeToken".to_string(),
            params: vec![token.to_string()],
        };
        if let Some((write, _)) = &mut self.connection {
            write
                .send(Message::Text(serde_json::to_string(&payload).unwrap()))
                .await
                .unwrap();
        }
        self.subsciptions.retain(|sub| sub.method != payload.method);
    }

    /// Reads incoming messages and prints them
    pub async fn listen(&mut self) {
        if let Some((_, read)) = &mut self.connection {
            println!("connection is live. listening...");
            while let Some(message) = read.next().await {
                match message {
                    Ok(Message::Text(text)) => {
                        println!("Received: {}", text);
                    }
                    Ok(Message::Close(close)) => {
                        println!("Connection closed: {:?}", close);
                        break;
                    }
                    Err(e) => {
                        eprintln!("Error: {}", e);
                        break;
                    }
                    _ => {}
                }
            }
        }
    }
}

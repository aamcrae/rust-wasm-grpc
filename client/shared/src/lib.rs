#[macro_export]
macro_rules! console_log {
    ($($t:tt)*) =>
        (web_sys::console::log_1(
            &wasm_bindgen::JsValue::from_str(&format!($($t)*))))
}

use std::time::Duration;

use bincode::{Decode, Encode, config};
use futures::StreamExt;
use futures::sink::SinkExt;
use gloo_worker::reactor::{ReactorScope, reactor};
use serde::{Deserialize, Serialize};
use tonic::Request;
use tonic_web_wasm_client::Client;

mod proto {
    tonic::include_proto!("streamer");
}

// Re-export JokeType from the protobuf definitions.
pub use proto::JokeType;

// Message(s) sent from main thread to worker.
#[derive(Encode, Decode, Serialize, Deserialize)]
pub struct Start {
    pub joke_type: i32,
}

// Messages sent from worker to main thread.
// The contents of the message are explicitly encoded via bincode as
// sending complex structures can cause the internal serialization to panic.
#[derive(Encode, Decode, Serialize, Deserialize)]
pub enum Message {
    Connected,
    Disconnected,
    Joke(Vec<u8>),
}

#[derive(Encode, Decode, Serialize, Deserialize)]
pub struct Joke {
    pub joke_type: i32,
    pub lines: Vec<String>,
}

#[reactor]
pub async fn JokeStream(mut scope: ReactorScope<Start, Message>) {
    // Wait for initial message from client, which indicates the type of jokes we want.
    while let Some(start) = scope.next().await {
        let server = String::from("http://127.0.0.1:8401");
        console_log!("Streamer worker started, connecting to {}", server);

        let mut conn = proto::streamer_client::StreamerClient::new(Client::new(server.clone()));
        // Main loop for connecting to server and then listening for streamed messages.
        loop {
			// Create the initial request parameter.
        	let req = Request::new(proto::Request {
            	joke_type: start.joke_type,
        	});

            // Connect to the server.
            match conn.jokes(req).await {
                Err(e) => {
                    console_log!("Server connection request: {}", e);
                    // Delay for a second to avoid slamming an unresponsive server.
                    gloo_timers::future::sleep(Duration::from_millis(1000)).await;
                }
                Ok(response) => {
                    // Connected successfully to server.
                    // Response will be a Stream of Jokes.
                    // Inform the client of the connection status change.
                    // Sending via the scope should always work, so use unwrap.
                    scope.send(Message::Connected).await.unwrap();
                    let mut joke_stream = response.into_inner();
                    // Get next joke until the server closes the stream.
                    while let Ok(Some(resp)) = joke_stream.message().await {
                        let joke = Joke {
                            joke_type: resp.joke_type,
                            lines: resp.lines,
                        };
                        // Send to main thread after encoding to byte vector.
                        scope
                            .send(Message::Joke(
                                bincode::encode_to_vec(joke, config::standard()).unwrap(),
                            ))
                            .await
                            .unwrap();
                    }
                    // Inform the client that the server has disconnected.
                    scope.send(Message::Disconnected).await.unwrap();
                }
            }
        }
    }
}

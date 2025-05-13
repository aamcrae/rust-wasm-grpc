/// WASM client for displaying jokes as they are streamed.
use bincode::config;
use futures::StreamExt;
use gloo_worker::Spawnable;
use std::time::Duration;
use wasm_bindgen::prelude::*;
use web_sys::Document;

use shared::{Joke, JokeStream, JokeType, Message, Start};

#[wasm_bindgen(start)]
pub fn start_client() {
    std::panic::set_hook(Box::new(console_error_panic_hook::hook));
    console_log::init().expect("could not initialize logger");

    // Spawn an async thread.
    wasm_bindgen_futures::spawn_local(async move {
        let doc = web_sys::window().unwrap().document().unwrap();
        // Element for displaying current server connection status.
        let status = doc.get_element_by_id("status").unwrap();
        // Start the web worker.
        let mut bridge = JokeStream::spawner().spawn("./worker.js");
        // Send the initial message - we'll listen to any type of joke.
        bridge.send_input(Start {
            joke_type: JokeType::Any.into(),
        });
        let indent_string = "&nbsp;".repeat(15);
        let indent = indent_string.as_str();
        loop {
            match bridge.next().await {
                Some(Message::Connected) => {
                    // Server is now connected, display the status
                    status.set_attribute("style", "color:green").unwrap();
                    status.set_inner_html("Connected");
                }
                Some(Message::Disconnected) => {
                    // Server has disconnected
                    status.set_attribute("style", "color:red").unwrap();
                    status.set_inner_html("Disconnected");
                }
                Some(Message::Joke(v)) => {
                    clear_old_joke(&doc);
                    // Give audience time to get ready for next joke.
                    gloo_timers::future::sleep(Duration::from_millis(1000)).await;
                    // Decode the joke from the message blob.
                    let (joke, _): (Joke, usize) =
                        bincode::decode_from_slice(&v, config::standard()).unwrap();
                    // If it is a knock-knock joke, indent every second line.
                    // This is a bit ugly, would be much nicer to have a separate CSS item for knock-knock jokes.
                    let f = if joke.joke_type == JokeType::KnockKnock as i32 {
                        ["", indent]
                    } else {
                        ["", ""]
                    };
                    for (i, l) in joke.lines.iter().enumerate() {
                        // Get the element for this line.
                        if let Some(ele) = doc.get_element_by_id(format!("r{}", i + 1).as_str()) {
                            ele.set_inner_html(format!("{}{}", f[i % 2], l).as_str());
                            // Comedy is all in the timing...
                            gloo_timers::future::sleep(Duration::from_millis(750)).await;
                        }
                    }
                }
                _ => {
                    // Should never happen
                    panic!("Server stopped sending jokes!");
                }
            }
        }
    });
}

// Clear out the old joke
fn clear_old_joke(doc: &Document) {
    for r in 1..20 {
        if let Some(ele) = doc.get_element_by_id(format!("r{}", r).as_str()) {
            ele.set_inner_html("");
        } else {
            return;
        }
    }
}

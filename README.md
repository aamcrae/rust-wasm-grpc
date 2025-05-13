# Rust WASM gRPC using a web worker

There are very few examples of Rust WASM clients that
use gRPC, and not any I have found that use web workers, so when
I figured out how to put all these together, I thought it would be useful
to publish a working example.

This example application combines Rust, WASM, gRPC, and web workers
all in a single working system (a very simple joke streaming service).

## Prerequisites

Ensure that the following items are installed:

 - [Protobuf compiler](http://grpc.io/docs/protoc-installation)
 - [wasm-bindgen](https://crates.io/crates/wasm-bindgen)
 - WASM Rust toolchain - use ```rustup target add wasm32-unknown-unknown```

## Building and running.

```build.sh``` is a shell script that will build all the binaries, and install
the WASM files into the ```www``` directory.

Run the server via:

```
cargo run -r --bin server
```

Open a browser tab and navigate to the URL displayed.

The server has a number of arguments that can be set to change the default port numbers etc.
Use ```cargo run -r --bin server -- --help``` to get help on the arguments.

## Points of Interest

The application is a simple streaming service that demonstrates:

 - Rust-based WASM modules, loaded by a simple web server and started by a script.
 - [Web-gRPC](https://github.com/grpc/grpc/blob/master/doc/PROTOCOL-WEB.md).  Due to limitations in the browser environment, there are differences between Web-gRPC and the full gRPC-over-HTTP2 protocol. This limits the streaming to server -> client; bi-directional streaming or client -> server streaming are not supported.
 - Web workers. The client offloads the gRPC handling to a WASM [web worker](https://developer.mozilla.org/en-US/docs/Web/API/Web_Workers_API/Using_web_workers) via [gloo-worker](https://docs.rs/gloo-worker/latest/gloo_worker). The protobuf decoding and server connection is managed completely within the web worker.

The web-worker/client interface operates by messages being sent to and from the worker, using the gloo-worker framework.
Internally, the framework serializes the messages using [bincode](https://docs.rs/bincode/latest/bincode/),
but I found with larger or more complex structs, it would often panic, so to work around this,
the messages are explicitly encoded and decoded via bincode before being sent, which seems to work much more reliably.

For a more complex system, messages from the client to the web worker could initiate separate RPCs to the server.
Potentially, more web workers could be used to offload work from the client.
These exercises are left to the reader.

## Supporting crates

This example relies on the following crates:

 - [prost](https://github.com/tokio-rs/prost)
 - [tonic-web-wasm-client](https://crates.io/crates/tonic-web-wasm-client)
 - [gloo-worker](https://docs.rs/gloo-worker/latest/gloo_worker)
 - [tonic](https://docs.rs/tonic/latest/tonic/)
 - [tokio](https://tokio.rs/)
 - [devserver](https://crates.io/crates/devserver)

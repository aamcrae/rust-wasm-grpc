/// Serves jokes via a streaming RPC.
use std::fs;
use std::io::{BufRead, BufReader};
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::sync::Arc;

use clap::Parser;
use rand::prelude::*;

use http::header::HeaderName;
use tokio::sync::mpsc;
use tokio::time::{Duration, sleep};
use tokio_stream::{Stream, wrappers::UnboundedReceiverStream};
use tonic::transport::Server;
use tonic::{Request, Response, Status};
use tonic_web::GrpcWebLayer;
use tower_http::cors::{AllowOrigin, CorsLayer};

use devserver_lib;

mod proto {
    tonic::include_proto!("streamer");
}

const DEFAULT_MAX_AGE: Duration = Duration::from_secs(24 * 60 * 60);
const DEFAULT_EXPOSED_HEADERS: [HeaderName; 3] = [
    HeaderName::from_static("grpc-status"),
    HeaderName::from_static("grpc-message"),
    HeaderName::from_static("grpc-status-details-bin"),
];
const DEFAULT_ALLOW_HEADERS: [HeaderName; 4] = [
    HeaderName::from_static("x-grpc-web"),
    HeaderName::from_static("content-type"),
    HeaderName::from_static("x-user-agent"),
    HeaderName::from_static("grpc-timeout"),
];

type ResponseStream = Pin<Box<dyn Stream<Item = Result<proto::Response, Status>> + Send>>;

struct Jokes {
    limericks: Vec<proto::Response>,
    knock_knock: Vec<proto::Response>,
}

struct Connection {
    jokes: Arc<Jokes>,
}

impl Connection {
    fn new(jokes: Jokes) -> Self {
        Connection {
            jokes: Arc::new(jokes),
        }
    }
}

#[tonic::async_trait]
impl proto::streamer_server::Streamer for Connection {
    type JokesStream = ResponseStream;
    async fn jokes(
        &self,
        request: Request<proto::Request>,
    ) -> Result<Response<Self::JokesStream>, Status> {
        let addr = request.remote_addr().unwrap();
        let req = request.into_inner();
        let joke_type = proto::JokeType::try_from(req.joke_type).unwrap();
        println!(
            "Client {:?} connected, requesting {} jokes",
            addr,
            joke_type.as_str_name(),
        );
        // Use a channel for the output stream.
        let (local_tx, local_rx) = mpsc::unbounded_channel();
        // Spawn a thread to send the stream to the client.
        let jokes = self.jokes.clone();
        tokio::spawn(async move {
            let mut rng = StdRng::from_os_rng();
            while !local_tx.is_closed() {
                let joke_list = match joke_type {
                    proto::JokeType::Limerick => &jokes.limericks,
                    proto::JokeType::KnockKnock => &jokes.knock_knock,
                    _ => {
                        // Randomly choose between limericks and knock-knock jokes.
                        if rng.random::<bool>() {
                            &jokes.limericks
                        } else {
                            &jokes.knock_knock
                        }
                    }
                };
                // Select a random joke from the list.
                if let Err(_) = local_tx.send(Ok(joke_list.choose(&mut rng).unwrap().clone())) {
                    break;
                }
                // Pause between jokes
                sleep(Duration::from_secs(10)).await;
            }
            println!("Streaming client closed");
        });

        // Convert the receiver side of the channel to an output
        // stream that is used by the RPC handler to send the streamed messages.
        let out_stream = UnboundedReceiverStream::new(local_rx);
        Ok(Response::new(Box::pin(out_stream) as Self::JokesStream))
    }
}

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Directory containing joke files
    #[arg(short, long, default_value = "jokes")]
    jokes: String,

    /// Directory containing HTML files
    #[arg(short, long, default_value = "www")]
    www: String,

    /// Server address
    #[arg(short, long, default_value = "127.0.0.1")]
    server: String,

    /// HTTP server port number
    #[arg(long, default_value_t = 8400)]
    port: u16,

    /// gRPC port number
    #[arg(long, default_value_t = 8401)]
    rpc: u16,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    let jokes = Jokes {
        limericks: read_jokes(
            proto::JokeType::Limerick,
            Path::new(&args.jokes).join("limericks.txt"),
        )?,
        knock_knock: read_jokes(
            proto::JokeType::KnockKnock,
            Path::new(&args.jokes).join("knock-knock.txt"),
        )?,
    };
    println!(
        "{} limericks, {} knock-knock jokes loaded",
        jokes.limericks.len(),
        jokes.knock_knock.len()
    );
    let server = args.server.clone();
    let port = args.port.into();
    // Spawn a thread to run devserver
    tokio::spawn(async move {
        println!("Devserver serving to {}:{} from {}", server, port, args.www);
        devserver_lib::run(&server, port, args.www.as_str(), true, "");
    });
    let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)), args.rpc);
    println!("Joke server at {}", addr);
    // Configure and run the server.
    Server::builder()
        .accept_http1(true)
        .layer(
            CorsLayer::new()
                .allow_origin(AllowOrigin::mirror_request())
                .allow_credentials(true)
                .max_age(DEFAULT_MAX_AGE)
                .expose_headers(DEFAULT_EXPOSED_HEADERS)
                .allow_headers(DEFAULT_ALLOW_HEADERS),
        )
        .layer(GrpcWebLayer::new())
        .add_service(proto::streamer_server::StreamerServer::new(
            Connection::new(jokes),
        ))
        .serve(addr)
        .await
        .unwrap();
    Ok(())
}

/// Read jokes from a file. A blank line separates each joke.
fn read_jokes(jt: proto::JokeType, file: PathBuf) -> Result<Vec<proto::Response>, std::io::Error> {
    let mut jokes = Vec::new();
    let file = fs::File::open(file)?;
    let mut joke = proto::Response::default();
    joke.joke_type = jt as i32;
    let reader = BufReader::new(file);
    for l in reader.lines().map_while(Result::ok) {
        // Empty line means end of the joke.
        if l.is_empty() {
            if !joke.lines.is_empty() {
                jokes.push(joke.clone());
                joke.lines.clear();
            }
        } else {
            joke.lines.push(l);
        }
    }
    // Trailing joke at end of file.
    if !joke.lines.is_empty() {
        jokes.push(joke);
    }
    Ok(jokes)
}

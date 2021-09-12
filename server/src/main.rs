use clap::{AppSettings, Clap};
use tokio::net::{TcpListener, TcpStream};
use tokio_util::codec::{FramedRead, FramedWrite};
use std::io;
use log::{info, warn, debug, LevelFilter};
use futures::stream::StreamExt;
use core::protocol::protocol::{ProtocolOperationCodec, Operation, Response, ProtocolResponseCodec};
use futures::SinkExt;


#[derive(Clap)]
#[clap(version = "1.0")]
#[clap(setting = AppSettings::ColoredHelp)]
struct Opts {
    #[clap(short, long, default_value = ".", about = "Data directory")]
    data_dir: String,
    #[clap(short, long, default_value = "127.0.0.1:4474", about = "Listen address")]
    listen_addr: String
}

async fn handle_client(stream: TcpStream, data_dir: String) {
    let (read, write) = stream.into_split();

    let mut framed_read = FramedRead::new(read, ProtocolOperationCodec);
    let mut framed_write = FramedWrite::new(write, ProtocolResponseCodec);

    while let Some(frame) = framed_read.next().await {
        let command = match frame {
            Ok(c) => c,
            Err(e) => {
                warn!("Error parsing command: {}", e);
                continue;
            }
        };

        debug!("Received command: {:?}", command);

        let response = match command {
            Operation::GetOperation { path } => {
                let content = tokio::fs::read_to_string(&*path).await.unwrap();

                Response::GetOperation {
                    content: content.into()
                }
            },
            Operation::ListOperation => {
                let mut entries = tokio::fs::read_dir(&data_dir).await.unwrap();
                let mut files = Vec::new();

                while let Some(entry) = entries.next_entry().await.unwrap() {
                    let path = entry.path();

                    if path.is_dir() {
                        continue
                    }

                    files.push(path.to_str().unwrap().to_string());
                }

                Response::ListOperation {
                    files
                }
            }
        };

        debug!("Sending a response: {:?}", response);

        if let Err(e) = framed_write.send(response).await {
            warn!("Error {}", e)
        }
    }
}


#[tokio::main]
async fn main() -> io::Result<()> {
    env_logger::builder()
        .filter_level(LevelFilter::Debug)
        .init();

    let opts = Opts::parse();

    let listener = TcpListener::bind(opts.listen_addr.clone()).await?;

    info!("Listening on {}", opts.listen_addr);

    loop {
        match listener.accept().await {
            Ok((socket, addr)) => {
                debug!("Client connected: {:?}", addr);

                tokio::spawn(handle_client(socket, opts.data_dir.clone()));
            },
            Err(e) => warn!("couldn't get client: {:?}", e),
        }
    }
}

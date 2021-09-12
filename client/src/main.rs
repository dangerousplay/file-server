use cursive::views::{Dialog, TextView, ListView, SelectView, TextArea};
use tokio::net::{TcpStream, ToSocketAddrs};
use std::io;
use std::borrow::Cow;
use futures::stream::StreamExt;
use tokio_util::codec::{FramedRead, FramedWrite};
use tokio::net::tcp::{WriteHalf, ReadHalf, OwnedReadHalf, OwnedWriteHalf};
use core::protocol::protocol::{ProtocolResponseCodec, ProtocolOperationCodec, Operation, ProtocolError, Response};
use futures::SinkExt;
use cursive::view::Nameable;
use std::sync::{Mutex, Arc};
use tokio::sync::RwLock;


pub struct Client {
    read: FramedRead<OwnedReadHalf, ProtocolResponseCodec>,
    write: FramedWrite<OwnedWriteHalf, ProtocolOperationCodec>
}

impl Client {
    pub async fn new<T: ToSocketAddrs>(addr: T) -> io::Result<Self> {
        let socket = TcpStream::connect(addr).await?;
        let (read, write) = socket.into_split();

        let read = FramedRead::new(read, ProtocolResponseCodec);
        let write = FramedWrite::new(write, ProtocolOperationCodec);

        Ok(Self {
            read,
            write
        })
    }

    pub async fn list_files(&mut self) -> Result<Vec<String>, ProtocolError> {
        self.write.send(Operation::ListOperation).await?;

        match self.read.next().await.unwrap()? {
            Response::ListOperation {
                files
            } => Ok(files),
            _ => unreachable!()
        }
    }

    pub async fn download_file<F: Into<Cow<'static, str>>>(&mut self, file: F) -> Result<String, ProtocolError> {
        self.write.send(Operation::GetOperation { path: file.into() }).await?;

        match self.read.next().await.unwrap()? {
            Response::GetOperation {
                content
            } => Ok(content.to_string()),
            _ => unreachable!()
        }
    }
}


#[tokio::main]
async fn main() -> io::Result<()> {
    let mut siv = cursive::default();

    let client = Arc::new(RwLock::new(Client::new("127.0.0.1:4474").await?));

    let files = client.write().await.list_files().await.unwrap().into_iter().map(|f| (f.clone(),f));

    let select_view = SelectView::<String>::new()
        .with_all(files)
        .on_submit(move |s, f: &str | {
            let f = f.to_owned();
            let cb = s.cb_sink().clone();
            let client = client.clone();

            tokio::spawn(async move {
                let content = client.write().await.download_file(f.clone()).await.unwrap();
                cb.send(Box::new(move |v|
                    v.add_layer(
                        Dialog::around(TextArea::new().content(content))
                            .title(f.clone())
                            .button("Close", |s| {
                                s.pop_layer();
                            })
                    )
                ));
            });
        });

    let dialog_name = "dialog";

    // Creates a dialog with a single "Quit" button
    siv.add_layer(Dialog::around(TextView::new("Files"))
        .title("Files on the server")
        .content(select_view)
        .button("Quit", |s| s.quit())
        .with_name(dialog_name)
    );

    // Starts the event loop.
    siv.run();

    Ok(())
}

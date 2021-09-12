A simple file transfer implementation

![](https://i.imgur.com/Nv4m79d.png)

# How to run

You need Cargo and Rust installed to compile the code.

```shell
# Running the server
$ cargo run --bin server

# Start the client after the server is running
$ cargo run --bin client
```

## Server options

```
OPTIONS:
    -d, --data-dir <DATA_DIR>          Data directory [default: .]
    -l, --listen-addr <LISTEN_ADDR>    Listen address [default: 127.0.0.1:4474]
```

# Protocol

Grammar is defined in the [protocol.pest](./core/src/protocol/protocol.pest) file.
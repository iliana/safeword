//! An example of why you might want Safeword: dropping a UnixListener doesn't delete the socket,
//! and starting the application again causes an error because the address is already in use. Our
//! application should delete the socket when it exits, but we may be interested in leaving the
//! socket around if the application does not exit cleanly.

extern crate safeword;
extern crate tokio;
extern crate tokio_uds;

use safeword::Safeword;
use std::fs;
use tokio::io;
use tokio::prelude::*;
use tokio_uds::UnixListener;

fn main() {
    let socket = UnixListener::bind("echo.sock").unwrap();
    match Safeword::default().run(
        socket
            .incoming()
            .map_err(|err| eprintln!("{:?}", err))
            .for_each(|stream| {
                let (reader, writer) = stream.split();
                tokio::spawn(io::copy(reader, writer).then(|result| {
                    if let Err(err) = result {
                        eprintln!("{:?}", err);
                    }
                    Ok(())
                }))
            }),
    ) {
        Ok(()) => {
            fs::remove_file("echo.sock").unwrap();
            eprintln!("application closed cleanly");
        }
        Err(err) => eprintln!("application unexpectedly stopped: {:?}", err),
    }
}

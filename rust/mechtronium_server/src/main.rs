extern crate clap;
extern crate tokio;

use clap::{App, Arg};
use mechtronium::transport::Constellation;
use mechtronium::error::Error;

use tokio::io::{self,AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use std::num::ParseIntError;
use tokio::sync::Mutex;
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(),Error> {
    let matches = opts().get_matches();

    let constellation = Constellation::new_standalone(Option::None);

    if let Option::Some(addrs) = matches.values_of("connect")
    {
        for addr in addrs
        {
            println!("connecting to {} ...", addr);
            let socket = TcpStream::connect(addr).await;
            match socket{
                Ok(socket) => {
                    let (mut rd, mut wr) = io::split(socket);
                    println!("connected: {}", addr);
                }
                Err(error) => {

                    println!("connection error: {}", error);
                }
            }
        }
    }


    if let Option::Some(port) = matches.value_of("listen")
    {
        println!("listening to port: {}",port);
        if let Ok(port) = port.parse::<usize>()
        {
            let mut listener = TcpListener::bind(format!("127.0.0.1:{}", port)).await?;
            println!("LISTENING FOR CONNECTIONS !!!");
            loop {
                let (mut socket, _) = listener.accept().await?;
                println!("RECEIVED SOCKET!!!");
            }
        }
        else {
            panic!("not a valid listening port '{}'",port );
        }
    }
    else {
        println!("no port specified");
    }


    Ok(())

}




pub fn opts()->App<'static, 'static>
{
    App::new("Mechtronium")
        .version("0.0.1")
        .author("Scott Williams <scott@mightydevco.com>")
        .about("Host a Mechtronium constellation.  Connect to other constellations.  Listen for connections from other constellations.")
        .arg(Arg::with_name("listen")
            .long("listen")
            .value_name("port")
            .help("Set the port for listening for socket connections")
            .takes_value(true))
        .arg(Arg::with_name("connect")
            .short("c")
            .long("connect")
            .value_name("connect")
            .multiple(true)
            .help("connect this mechtronium constellation to another")
            .takes_value(true))
}





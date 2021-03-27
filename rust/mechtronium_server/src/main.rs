extern crate clap;

use clap::{App, Arg};
use mechtronium::transport::Constellation;


fn main() {
    let matches = opts().get_matches();

    let constellation = Constellation::new_standalone(Option::None);

    if let Option::Some(port) = matches.value_of("listen")
    {
        println!("listening to port: {}",port)
    }
    else {
        println!("no port specified");
    }

    if let Option::Some(connections) = matches.values_of("connect")
    {
        for connect in connections
        {
           println!("connect to {}",connect);
        }
    }

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





#![crate_type = "bin"]

extern crate rustc_serialize;
extern crate atty;
extern crate docopt;
extern crate protobuf;
extern crate serde;
extern crate serde_protobuf;
extern crate serde_value;
extern crate serde_json;
extern crate stream_delimit;

mod fdset_discovery;
mod error;
mod decode;
#[macro_use]
mod macros;

use docopt::Docopt;
use decode::PqrsDecoder;
use stream_delimit::StreamDelimiter;
use std::fs::File;
use std::io::{self, Read, BufReader, Write, stderr};
use std::process;

const VERSION: &'static str = env!("CARGO_PKG_VERSION");

const USAGE: &'static str = "
pq - protobuf to json

Usage:
  pq [<infile>] [--msgtype=<msgtype>] [--stream=<delim>] [--trail=<delim>]
  pq (--help | --version)

Options:
  --stream=<delim>      Stream delimiter e.g. \"varint\", \"leb128\"
  --trail=<num>         Number of chars to chomp from tail
  --msgtype=<msgtype>   Message type e.g. com.example.Type
  --help                Show this screen.
  --version             Show version.
";

#[derive(Debug, RustcDecodable)]
struct Args {
    pub arg_infile: Option<String>,
    pub flag_msgtype: Option<String>,
    pub flag_stream: Option<String>,
    pub flag_trail: Option<usize>,
    flag_version: bool,
}

fn main() {
    let mut stdout = io::stdout();
    let stdin = io::stdin();

    let out_is_tty = atty::is(atty::Stream::Stdout);

    let args: Args = Docopt::new(USAGE)
        .and_then(|d| d.version(Some(String::from(VERSION))).decode())
        .unwrap_or_else(|e| e.exit());

    let pqrs_decoder = match PqrsDecoder::new(args.flag_msgtype) {
        Ok(x) => x,
        Err(e) => errexit!(e),
    };

    let mut infile: Box<Read> = match args.arg_infile {
        Some(x) => {
            let file = match File::open(&x) {
                Ok(x) => x,
                Err(e) => errexit!(e),
            };
            Box::new(BufReader::new(file))
        }
        None => {
            if atty::is(atty::Stream::Stdin) {
                writeln!(stdout,
                         "pq expects input to be piped from stdin - run with --help for more info")
                        .unwrap();
                process::exit(0);
            }
            Box::new(stdin.lock())
        }
    };

    if let Some(lead_delim) = args.flag_stream {
        let delim = StreamDelimiter::new(&lead_delim, &mut infile, args.flag_trail);
        for chunk in delim {
            match pqrs_decoder.decode_message(&chunk, &mut stdout.lock(), out_is_tty) {
                Ok(_) => (),
                Err(e) => errexit!(e),
            }
            if out_is_tty {
                writeln!(&mut stdout, "").unwrap();
            }
        }
    } else {
        let mut buf = Vec::new();
        match infile.read_to_end(&mut buf) {
            Ok(_) => (),
            Err(e) => errexit!(e),
        }
        match pqrs_decoder.decode_message(&buf, &mut stdout.lock(), out_is_tty) {
            Ok(_) => (),
            Err(e) => errexit!(e),
        }
    }
}

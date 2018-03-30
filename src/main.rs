//#[macro_use]
extern crate failure;
#[macro_use]
extern crate structopt;

use failure::Error;
use structopt::StructOpt;

#[derive(StructOpt, Debug)]
#[structopt(name = "rlhcbfix")]
struct Opt {
    /// Verbose mode
    #[structopt(short = "v", long = "verbose")]
    verbose: bool,
}

fn run() -> Result<(), Error> {
    let _opt = Opt::from_args();

    Ok(())
}

fn main() {
    run().unwrap()
}

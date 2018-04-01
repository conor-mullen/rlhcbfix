//#[macro_use]
extern crate failure;
#[macro_use]
extern crate structopt;
extern crate rlhcbfix;
#[macro_use]
extern crate log;
extern crate pretty_env_logger;

use std::env;
use std::thread;
use std::time::Duration;

use failure::Error;
use structopt::StructOpt;

use rlhcbfix::manage_rl_threads;

#[derive(StructOpt, Debug)]
#[structopt(name = "rlhcbfix")]
struct Opt {
    /// Verbose mode
    #[structopt(short = "v", long = "verbose")]
    verbose: bool,
    /// Polling interval (in seconds)
    #[structopt(short = "p", long = "poll", default_value = "1")]
    poll_interval: u64,
    /// Settling period (in seconds)
    #[structopt(short = "s", long = "settle", default_value = "15")]
    settling_period: u64,
}

fn run() -> Result<(), Error> {
    let opt: Opt = Opt::from_args();
    if opt.verbose {
        env::set_var("RLHCB_LOG", "rlhcbfix=debug");
        pretty_env_logger::try_init_custom_env("RLHCB_LOG")?;
    } else {
        pretty_env_logger::try_init()?;
    }
    let poll_interval = Duration::from_secs(opt.poll_interval);
    let settling_period = Duration::from_secs(opt.settling_period);
    let retry_period = Duration::from_secs(5);

    loop {
        match manage_rl_threads(poll_interval, settling_period) {
            Err(e) => match e {
                rlhcbfix::Error::NoProcess => {}
                rlhcbfix::Error::Windows(ref we) if we.code() == 31 => {}
                e @ _ => {
                    Err(e)?;
                }
            },
            Ok(_) => {}
        }
        warn!(
            "No Rocket League process found. Retrying in {} seconds.",
            retry_period.as_secs()
        );
        thread::sleep(retry_period);
    }
}

fn main() {
    run().unwrap()
}

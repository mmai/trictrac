// Application.
pub mod app;
mod game_runner;

use anyhow::Result;
use app::{App, AppArgs};
use std::io;

// see pico-args example at https://github.com/RazrFalcon/pico-args/blob/master/examples/app.rs
const HELP: &str = "\
Trictrac CLI

USAGE:
  trictrac-cli [OPTIONS]

FLAGS:
  -h, --help            Prints help information

OPTIONS:
  --seed SEED         Sets the random generator seed
  --bot STRATEGY_BOT  Add a bot player with strategy STRATEGY, a second bot may be added to play against the first : --bot STRATEGY_BOT1,STRATEGY_BOT2

ARGS:
  <INPUT>
";

fn main() -> Result<()> {
    env_logger::init();
    let args = match parse_args() {
        Ok(v) => v,
        Err(e) => {
            eprintln!("Error: {}.", e);
            std::process::exit(1);
        }
    };
    // println!("{:#?}", args);

    // Create an application.
    let mut app = App::new(args);

    // Start the main loop.
    while !app.should_quit {
        println!("whot?>");
        let mut input = String::new();
        let _bytecount = io::stdin().read_line(&mut input)?;
        app.input(input.trim());
    }

    Ok(())
}

fn parse_args() -> Result<AppArgs, pico_args::Error> {
    let mut pargs = pico_args::Arguments::from_env();

    // Help has a higher priority and should be handled separately.
    if pargs.contains(["-h", "--help"]) {
        print!("{}", HELP);
        std::process::exit(0);
    }

    let args = AppArgs {
        // Parses an optional value that implements `FromStr`.
        seed: pargs.opt_value_from_str("--seed")?,
        bot: pargs.opt_value_from_str("--bot")?,
        // Parses an optional value from `&str` using a specified function.
        // width: pargs.opt_value_from_fn("--width", parse_width)?.unwrap_or(10),
    };

    // It's up to the caller what to do with the remaining arguments.
    let remaining = pargs.finish();
    if !remaining.is_empty() {
        eprintln!("Warning: unused arguments left: {:?}.", remaining);
    }

    Ok(args)
}

// fn parse_width(s: &str) -> Result<u32, &'static str> {
//     s.parse().map_err(|_| "not a number")
// }

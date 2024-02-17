// Application.
pub mod app;

use anyhow::Result;
use app::App;
use std::io;

fn main() -> Result<()> {
    // Create an application.
    let mut app = App::new();

    // Start the main loop.
    while !app.should_quit {
        println!("whot?>");
        let mut input = String::new();
        let _bytecount = io::stdin().read_line(&mut input)?;
        app.input(input.trim());
    }

    Ok(())
}

mod backends;
mod grabs;
mod handlers;
mod input;
mod layout;
mod renderer;
mod state;
mod window;

use anyhow::Result;
pub use state::PocoWM;

fn run() -> Result<()> {
    let mut pocowm = PocoWM::new()?;
    pocowm.init_winit()?;
    pocowm.run()?;

    Ok(())
}

fn main() -> Result<()> {
    // tracing_subscriber::fmt().init();
    if let Err(err) = run() {
        eprintln!("{:?}", err);
        std::process::exit(1);
    }

    Ok(())
}

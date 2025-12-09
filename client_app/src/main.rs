mod ui;
mod app;

use anyhow::Result;
use app::App;
use tracing::error;

fn main() -> Result<()> {
    tracing_subscriber::fmt::init();
    let async_runtime = tokio::runtime::Runtime::new()?;
    //let handle = async_runtime.handle().clone(); 
    let result = App::run_native(async_runtime);
    
    if let Err(e) = result {
        error!("Error running app: {}", e);
    };
    Ok(())
}
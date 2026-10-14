pub mod services;
mod startup;
mod ui;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?;

    let services = runtime.block_on(startup::initialize())?;
    ui::launch(services, runtime.handle().clone());
    Ok(())
}

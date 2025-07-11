use cursorup::Result;

#[tokio::main]
async fn main() -> Result<()> {
    if let Err(e) = cursorup::run().await {
        eprintln!("Application error: {e}");
        std::process::exit(1);
    }
    Ok(())
}

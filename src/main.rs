#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    ani_cli::runtime::run().await
}

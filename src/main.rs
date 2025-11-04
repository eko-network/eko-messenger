#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    eko_messenger::run().await
}

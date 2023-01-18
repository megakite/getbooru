#[tokio::main]
async fn main() {
    getbooru::build(std::env::args()).await.unwrap_or_else(|e| {
        println!("Runtime error: {e}");
    })
}

#[tokio::main]
async fn main() {
    let user_id = dotenv::var("user_id").unwrap();
    let api_key = dotenv::var("api_key").unwrap();

    let file_tags = "tags.txt";
    let common_tag = "solo";
    let folder = "saved";

    getbooru::get_posts(file_tags, common_tag, &api_key, &user_id, folder)
        .await
        .unwrap_or_else(|e| {
            println!("Runtime error: {e}");
        })
}

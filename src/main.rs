fn show_help() {
    println!("Example usage:");
    println!("    getbooru get favorites // Get all of your favorites into current directory");
    println!("    getbooru add favorites by urls.txt // Add urls in links.txt to your favorites");
    println!("    getbooru get posts from 6 to 9 api // Get posts in page 6-9, using API");
    println!("    getbooru get posts with 1boy into dir // Get posts with tag '1boy' into ./dir/");
    println!(
        "Note: the 'api' switch can be combined with 'quick', which speeds up the progress but alternates saved file names"
    );
}

#[tokio::main]
async fn main() {
    let mut opt = getbooru::Session::options();

    if let Ok(api_key) = dotenv::var("api_key") {
        opt.api_key(api_key);
    }
    if let Ok(user_id) = dotenv::var("user_id") {
        opt.user_id(user_id);
    }
    if let Ok(pass_hash) = dotenv::var("pass_hash") {
        opt.pass_hash(pass_hash);
    }
    if let Ok(fringe_benefits) = dotenv::var("fringeBenefits") {
        opt.fringe_benefits(fringe_benefits);
    }

    let mut args = std::env::args();

    match args.nth(1) {
        Some(s) if s == "get" => match args.next() {
            Some(s) if s == "posts" => {
                opt.get_posts();
            }
            Some(s) if s == "favorites" => {
                opt.get_favorites();
            }
            _ => {
                show_help();
                return;
            }
        },
        Some(s) if s == "add" => match args.next() {
            Some(s) if s == "favorites" => {
                opt.add_favorites();
            }
            _ => {
                show_help();
                return;
            }
        },
        Some(_) => {
            show_help();
            return;
        }
        None => {}
    }

    while let Some(s) = args.next() {
        if s == "from" {
            if let Some(n) = args.next() {
                opt.start(n.parse::<u64>().unwrap());
                continue;
            }
        }
        if s == "to" {
            if let Some(n) = args.next() {
                opt.end(n.parse::<u64>().unwrap());
                continue;
            }
        }
        if s == "by" {
            if let Some(p) = args.next() {
                opt.file(p);
                continue;
            }
        }
        if s == "into" {
            if let Some(p) = args.next() {
                opt.folder(p);
                continue;
            }
        }
        if s == "with" {
            if let Some(p) = args.next() {
                opt.tags(p);
                continue;
            }
        }
        if s == "api" {
            opt.api(true);
            continue;
        }
        if s == "quick" {
            opt.quick(true);
            continue;
        }

        show_help();
        return;
    }

    opt.create().start().await.unwrap();
}

pub fn show_help() {
    println!("Example usage:");
    println!("getbooru get favorites // Get all of your favorites into current directory");
    println!("getbooru add favorites by urls.txt // Add urls in links.txt to your favorites");
    println!("getbooru get posts from 6 to 9 api // Get posts in page 6-9 using Gelbooru API");
    println!(
        "getbooru get posts with game_cg into dir // Get posts with tag 'game_cg' into ./dir/"
    );
}

#[tokio::main]
async fn main() {
    let mut opt = getbooru::Session::options();

    if let Some(api_key) = dotenv::var("api_key").ok() {
        opt.api_key(api_key);
    }
    if let Some(user_id) = dotenv::var("user_id").ok() {
        opt.user_id(user_id);
    }
    if let Some(pass_hash) = dotenv::var("pass_hash").ok() {
        opt.pass_hash(pass_hash);
    }
    if let Some(fringe_benefits) = dotenv::var("fringeBenefits").ok() {
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
            Some(_) | None => {
                show_help();
                return;
            }
        },
        Some(s) if s == "add" => match args.next() {
            Some(s) if s == "favorites" => {
                opt.add_favorites();
            }
            Some(_) | None => {
                show_help();
                return;
            }
        },
        Some(_) | None => {
            show_help();
            return;
        }
    }

    while let Some(s) = args.next() {
        if s == "from" {
            if let Some(n) = args.next() {
                opt.begin(n.parse::<u64>().unwrap());
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
            opt.use_api(true);
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

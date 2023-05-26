const HELP: &str = "Example usage:
    getbooru get favorites # (WebDriver needed) Get all of your favorites into current directory
    getbooru add favorites by urls.txt # (WebDriver needed) Add urls in links.txt to your favorites
    getbooru get posts from 6 to 9 api # Get posts in page 6-9, using API
    getbooru get posts with 1boy into dir # Get posts with tag '1boy' into ./dir/
Note: 'api' can be combined with 'quick', which speeds up the progress but alternates file names.";

#[tokio::main]
async fn main() {
    let mut opt = getbooru::Session::options();

    if let Ok(api_key) = dotenv::var("api_key") {
        opt.api_key(api_key.as_str());
    }
    if let Ok(user_id) = dotenv::var("user_id") {
        opt.user_id(user_id.as_str());
    }
    if let Ok(pass_hash) = dotenv::var("pass_hash") {
        opt.pass_hash(pass_hash.as_str());
    }
    if let Ok(fringe_benefits) = dotenv::var("fringeBenefits") {
        opt.fringe_benefits(fringe_benefits.as_str());
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
                println!("{HELP}");
                return;
            }
        },
        Some(s) if s == "add" => match args.next() {
            Some(s) if s == "favorites" => {
                opt.add_favorites();
            }
            _ => {
                println!("{HELP}");
                return;
            }
        },
        Some(_) => {
            println!("{HELP}");
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
                opt.file(p.as_str());
                continue;
            }
        }
        if s == "into" {
            if let Some(p) = args.next() {
                opt.folder(p.as_str());
                continue;
            }
        }
        if s == "with" {
            if let Some(p) = args.next() {
                opt.tags(p.as_str());
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

        println!("{HELP}");
        return;
    }

    opt.create().start().await.unwrap();
}

pub fn show_help() {
    println!("Example usage:");
    println!("getbooru get favorites // get all of your favorites into current directory");
    println!("getbooru add favorites by urls.txt // add posts in links.txt to your favorites");
    println!("getbooru get posts from 6 to 9 api quick // get posts in page 6-9, using api and quick mode");
    println!("getbooru get posts by tags.txt with game_cg into saved // y'know what it means");
}

#[tokio::main]
async fn main() {
    let mut opt = getbooru::Session::options();

    let api_key = dotenv::var("api_key").unwrap();
    let user_id = dotenv::var("user_id").unwrap();
    let pass_hash = dotenv::var("pass_hash").unwrap();
    let fringe_benefits = dotenv::var("fringeBenefits").unwrap();
    opt.api_key(&api_key);
    opt.user_id(&user_id);
    opt.pass_hash(&pass_hash);
    opt.fringe_benefits(&fringe_benefits);

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
                std::process::exit(1);
            }
        },
        Some(s) if s == "add" => match args.next() {
            Some(s) if s == "favorites" => {
                opt.add_favorites();
            }
            Some(_) | None => {
                show_help();
                std::process::exit(1);
            }
        },
        Some(_) => {
            show_help();
            std::process::exit(1);
        },
        None => {}
    }

    let mut error = false;
    loop {
        match args.next() {
            Some(s) if s == "from" => match args.next() {
                Some(n) => {
                    opt.begin(n.parse::<u64>().unwrap());
                }
                None => error = true,
            },
            Some(s) if s == "to" => match args.next() {
                Some(n) => {
                    opt.end(n.parse::<u64>().unwrap());
                }
                None => error = true,
            },
            Some(s) if s == "by" => match args.next() {
                Some(p) => {
                    opt.file(&p);
                }
                None => error = true,
            },
            Some(s) if s == "into" => match args.next() {
                Some(p) => {
                    opt.folder(&p);
                }
                None => error = true,
            },
            Some(s) if s == "with" => match args.next() {
                Some(p) => {
                    opt.tags(&p);
                }
                None => error = true,
            },
            Some(s) if s == "api" => {
                opt.use_api(true);
            }
            Some(s) if s == "quick" => {
                opt.quick(true);
            }
            Some(_) => error = true,
            None => {
                break;
            }
        }

        if error {
            show_help();
            std::process::exit(1);
        }
    }

    opt.create().start().await.unwrap();
}

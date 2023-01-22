pub fn show_help() {
    println!("Example usage:");
    println!("getbooru get favorites into saved // get all of your favorites into ./saved/");
    println!(
        "getbooru add favorites by urls.txt // add posts listed in links.txt to your favorites"
    );
    println!("getbooru get post by tags.txt with game_cg from 6 to 9 // y'know what it means");
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

    std::fs::File::options();
    let mut args = std::env::args();
    match args.nth(1) {
        Some(s) if s == "get" => match args.next() {
            Some(s) if s == "post" => {
                opt.get_post();
            }
            Some(s) if s == "favorites" => {
                opt.get_favorites();
            }
            Some(_) | None => {
                getbooru::show_help();
                std::process::exit(1);
            }
        },
        Some(s) if s == "add" => match args.next() {
            Some(s) if s == "favorites" => {
                opt.add_favorites();
            }
            Some(_) | None => {
                getbooru::show_help();
                std::process::exit(1);
            }
        },
        Some(_) | None => getbooru::show_help(),
    }

    loop {
        match args.next() {
            Some(s) if s == "from" => match args.next() {
                Some(n) => {
                    opt.begin(n.parse::<u64>().unwrap());
                }
                None => {
                    getbooru::show_help();
                    std::process::exit(1);
                }
            },
            Some(s) if s == "to" => match args.next() {
                Some(n) => {
                    opt.end(n.parse::<u64>().unwrap());
                }
                None => {
                    getbooru::show_help();
                    std::process::exit(1);
                }
            },
            Some(s) if s == "by" => match args.next() {
                Some(p) => {
                    opt.file(&p);
                }
                None => {
                    getbooru::show_help();
                    std::process::exit(1);
                }
            },
            Some(s) if s == "into" => match args.next() {
                Some(p) => {
                    opt.folder(&p);
                }
                None => {
                    getbooru::show_help();
                    std::process::exit(1);
                }
            },
            Some(s) if s == "with" => match args.next() {
                Some(p) => {
                    opt.tags(&p);
                }
                None => {
                    getbooru::show_help();
                    std::process::exit(1);
                }
            },
            Some(s) if s == "noapi" => {
                opt.use_api(false);
            }
            Some(s) if s == "quick" => {
                opt.quick(true);
            }
            Some(_) => {
                getbooru::show_help();
                std::process::exit(1);
            }
            None => {
                break;
            }
        }
    }

    opt.start().await.unwrap();
}

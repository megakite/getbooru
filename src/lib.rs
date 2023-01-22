use std::{
    error::Error,
    fmt::Debug,
    fs::{self, File},
    io::{self, Read, Write},
    path::Path,
};

const PID_STEP_VIEW: u64 = 50;
const PID_STEP_LIST: u64 = 42;
const TITLE_LENGTH_LIMIT: usize = 100;

enum Action {
    GetPost,
    GetFavorites,
    AddFavorites,
}

impl Debug for Action {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Action::GetPost => "GetPost",
            Action::GetFavorites => "GetFavorites",
            Action::AddFavorites => "AddFavorites",
        })
    }
}

pub struct SessionOptions {
    api_key: Option<String>,
    user_id: Option<String>,
    pass_hash: Option<String>,
    fringe_benefits: Option<String>,
    action: Action,
    begin: Option<u64>,
    end: Option<u64>,
    tags: Option<String>,
    file: Option<String>,
    folder: Option<String>,
    use_api: bool,
    quick: bool,
}

impl SessionOptions {
    pub fn user_id(&mut self, s: &str) -> &mut Self {
        self.user_id = Some(s.to_string());
        self
    }
    pub fn api_key(&mut self, s: &str) -> &mut Self {
        self.api_key = Some(s.to_string());
        self
    }
    pub fn pass_hash(&mut self, s: &str) -> &mut Self {
        self.pass_hash = Some(s.to_string());
        self
    }
    pub fn fringe_benefits(&mut self, s: &str) -> &mut Self {
        self.fringe_benefits = Some(s.to_string());
        self
    }
    pub fn get_favorites(&mut self) -> &mut Self {
        self.action = Action::GetFavorites;
        self
    }
    pub fn get_post(&mut self) -> &mut Self {
        self.action = Action::GetPost;
        self
    }
    pub fn add_favorites(&mut self) -> &mut Self {
        self.action = Action::AddFavorites;
        self
    }
    pub fn begin(&mut self, n: u64) -> &mut Self {
        self.begin = Some(n);
        self
    }
    pub fn end(&mut self, n: u64) -> &mut Self {
        self.end = Some(n);
        self
    }
    pub fn tags(&mut self, s: &str) -> &mut Self {
        self.tags = Some(s.to_string());
        self
    }
    pub fn file(&mut self, s: &str) -> &mut Self {
        self.file = Some(s.to_string());
        self
    }
    pub fn folder(&mut self, s: &str) -> &mut Self {
        self.folder = Some(s.to_string());
        self
    }
    pub fn use_api(&mut self, b: bool) -> &mut Self {
        self.use_api = b;
        self
    }
    pub fn quick(&mut self, b: bool) -> &mut Self {
        self.quick = b;
        self
    }

    pub async fn start(self) -> Result<(), Box<dyn Error>> {
        self._start().await
    }
    async fn _start(self) -> Result<(), Box<dyn Error>> {
        Session::start(self).await
    }
}

impl Default for SessionOptions {
    fn default() -> Self {
        Self {
            api_key: None,
            user_id: None,
            pass_hash: None,
            fringe_benefits: None,
            action: Action::GetPost,
            begin: None,
            end: None,
            tags: None,
            file: None,
            folder: None,
            use_api: true,
            quick: false,
        }
    }
}

impl Debug for SessionOptions {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SessionOptions")
            .field("api_key", &self.api_key)
            .field("user_id", &self.user_id)
            .field("pass_hash", &self.pass_hash)
            .field("fringe_benefits", &self.fringe_benefits)
            .field("action", &self.action)
            .field("begin", &self.begin)
            .field("end", &self.end)
            .field("tags", &self.tags)
            .field("file", &self.file)
            .field("folder", &self.folder)
            .field("use_api", &self.use_api)
            .field("quick", &self.quick)
            .finish()
    }
}

pub struct Session {
    options: SessionOptions,
}

impl Session {
    pub fn options() -> SessionOptions {
        SessionOptions::default()
    }

    fn extract_url_from_meta(res: &str) -> &str {
        let begin = res.find("content=\"https://").unwrap() + "content=\"".len();
        let end = res.find("\" />\n\t\t<meta name=\"twitter:card\"").unwrap();

        &res[begin..end]
    }

    fn extract_title_from_meta(res: &str) -> std::borrow::Cow<str> {
        let begin = res.find("<title>").unwrap() + "<title>".len();
        let mut end = res.find("</title>").unwrap();
        if end - begin > TITLE_LENGTH_LIMIT {
            end = begin + TITLE_LENGTH_LIMIT;
        }
        let re = regex::Regex::new("[/\\?%*:|\"<>]").unwrap();
        let title = re.replace_all(&res[begin..end], "_");

        title
    }

    async fn init_webdriver_client(
        &self,
    ) -> Result<fantoccini::Client, fantoccini::error::CmdError> {
        let c = fantoccini::ClientBuilder::native()
            .connect("http://localhost:4444")
            .await
            .expect("failed to connect to WebDriver");

        print!("Entering home page...");
        io::stdout().flush()?;
        c.goto("https://gelbooru.com/index.php").await?;

        println!("adding cookies...");
        if let Some(ui) = &self.options.user_id {
            c.add_cookie(fantoccini::cookies::Cookie::new("user_id", ui.to_owned()))
                .await?;
        }
        if let Some(ph) = &self.options.pass_hash {
            c.add_cookie(fantoccini::cookies::Cookie::new("pass_hash", ph.to_owned()))
                .await?;
        }
        if let Some(fb) = &self.options.fringe_benefits {
            c.add_cookie(fantoccini::cookies::Cookie::new("fringe_benefits", fb.to_owned()))
                .await?;
        }

        Ok(c)
    }

    async fn download_noapi(id: &str) -> Result<(), Box<dyn Error>> {
        let src = String::from("https://gelbooru.com/index.php?page=post&s=view&id=") + id;
        let c = reqwest::Client::builder().build()?;

        print!("entering {id} ...");
        io::stdout().flush()?;
        let res = c.get(&src).send().await?.text().await?;

        let url = Self::extract_url_from_meta(&res);
        let title = Self::extract_title_from_meta(&res);
        let folder = String::from("saved");
        let extention = Path::new(url).extension().unwrap().to_str().unwrap();

        let path_string = folder + "/" + id + " " + title.as_ref() + "." + extention;
        if Path::new(&path_string).exists() {
            println!("already exists, skipping.");
            return Ok(());
        }

        print!("downloading...");
        io::stdout().flush()?;
        let img_bytes = c.get(url).send().await?.bytes().await?;
        File::create(&path_string)?.write(&img_bytes)?;
        println!("complete.");

        Ok(())
    }

    async fn get_list_noapi(
        client: &reqwest::Client,
        token: &str,
    ) -> Result<(), Box<dyn Error>> {
        let base =
            String::from("https://gelbooru.com/index.php?page=") + page_type + "&s=list&" + token;
        let downloaded: Vec<_> = fs::read_dir("saved/")?.collect();

        for page in init_page.. {
            print!("Entering {page_type}, page {page}...");
            std::io::stdout().flush()?;

            let pid = (page - 1) * PID_STEP_LIST;
            let url = base.to_owned() + "&pid=" + pid.to_string().as_str();

            println!("getting elements...");

            let res = client.get(url).send().await?.text().await?;
            let list = scraper::Html::parse_document(&res);
            let selector = scraper::Selector::parse("article.thumbnail-preview a").unwrap();
            let a_s: Vec<_> = list.select(&selector).collect();
            if a_s.is_empty() {
                println!("No elements present. ");

                break;
            }

            'outer: for a in a_s {
                print!("Extracting information...");
                io::stdout().flush()?;

                let src = a.value().attr("href").unwrap().to_string();
                let begin = src.find("&id=").unwrap() + "&id=".len();
                let end = src.find("&tags=").unwrap_or(src.len() + 1);
                let id = &src[begin..end];

                for file in &downloaded {
                    let name = file.as_ref().unwrap().file_name();
                    if name.to_str().unwrap().starts_with(id) {
                        println!("{id} already exists, skipping.");
                        continue 'outer;
                    }
                }

                Self::download_noapi(id).await?;
            }
        }

        Ok(())
    }

    async fn get_view(
        client: &fantoccini::Client,
        token: &str,
    ) -> Result<(), Box<dyn Error>> {
        let base =
            String::from("https://gelbooru.com/index.php?page=") + page_type + "&s=view&" + token;
        let downloaded: Vec<_> = fs::read_dir("saved/")?.collect();

        for page in init_page.. {
            print!("Entering {page_type}, page {page}...",);
            io::stdout().flush()?;

            let pid = (page - 1) * PID_STEP_VIEW;
            let url = base.to_owned() + "&pid=" + pid.to_string().as_str();
            client.goto(&url).await?;

            println!("getting elements...");

            let a_s = client
                .find_all(fantoccini::Locator::Css("span.thumb a:first-child"))
                .await?;
            if a_s.is_empty() {
                println!("No elements present. ");

                break;
            }

            'outer: for a in a_s {
                print!("Extracting information...");
                io::stdout().flush()?;

                let src = a.attr("href").await?.unwrap();
                let begin = src.find("&id=").unwrap() + "&id=".len();
                let end = src.find("&tags=").unwrap_or(src.len() + 1);
                let id = &src[begin..end];

                for file in &downloaded {
                    let name = file.as_ref().unwrap().file_name();
                    if name.to_str().unwrap().starts_with(id) {
                        println!("{id} already exists, skipping.");
                        continue 'outer;
                    }
                }

                Self::download_noapi(id).await?;
            }
        }

        Ok(())
    }

    async fn add_to_favorites(&self) -> Result<(), Box<dyn Error>> {
        println!("Start adding urls to favorites...");

        let client = Self::init_webdriver_client(&self).await?;

        let mut buf = String::new();
        if let Some(f) = &self.options.file {
            File::open(f)?.read_to_string(&mut buf)?;
        };
        for url in buf.lines() {
            print!("Entering {url} ...");
            io::stdout().flush()?;

            client.goto(url).await?;

            match client
                .find(fantoccini::Locator::Css("h4#scrollebox a:nth-child(3)"))
                .await
            {
                Ok(t) if t.text().await? == "Favorite" => {
                    println!("adding to favorites.");
                    t.click().await?;
                }
                Ok(_) => {
                    println!("already in favorites, skipping.");
                }
                Err(e) => {
                    println!("problem finding favorite button, skipping: {e}.");
                }
            };
        }

        println!("Finished adding to favorites.");
        Ok(())
    }

    async fn get_favorites(
        &self
    ) -> Result<(), Box<dyn Error>> {
        println!("Start getting favorites...");

        let client = Self::init_webdriver_client(&self).await?;

        let token = String::from("id=") + &self.options.user_id;
        Self::get_view(&self, client).await?;

        println!("Finished getting favorites.");

        Ok(())
    }

    async fn get_posts_noapi(
        init_page: u64,
        path_tags: &str,
        common_tag: &str,
        pass_hash: &str,
        user_id: &str,
        fringe_benefits: &str,
    ) -> Result<(), Box<dyn Error>> {
        println!("Start getting by tags...");

        let cookie = String::from("pass_hash=")
            + pass_hash
            + "; user_id="
            + user_id
            + "; fringeBenefits="
            + fringe_benefits;
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert(
            reqwest::header::COOKIE,
            reqwest::header::HeaderValue::from_str(&cookie).unwrap(),
        );
        let client = reqwest::Client::builder()
            .default_headers(headers)
            .build()?;

        let mut file_tags = File::open(path_tags)?;
        let mut buf = String::new();
        file_tags.read_to_string(&mut buf)?;
        for tag in buf.lines() {
            if tag.starts_with("#") {
                continue;
            }
            println!("Current tags: {tag}+{common_tag}");

            let token = String::from("tags=") + tag + "+" + common_tag;
            Self::get_list_noapi(&client, token.as_str()).await?;
        }

        println!("Finished getting all tags.");

        Ok(())
    }

    async fn get_posts(
        &self
    ) -> Result<(), Box<dyn Error>> {
        println!("Start getting by tags...");

        let base = String::from("https://gelbooru.com/index.php?page=dapi&s=post&q=index&api_key=")
            + self.options.api_key.as_ref().unwrap().as_str()
            + "&user_id="
            + self.options.user_id.as_ref().unwrap().as_str();

        let mut file_tags = File::open(self.options.file.as_ref().unwrap())?;
        let mut buf = String::new();
        file_tags.read_to_string(&mut buf)?;
        for tag in buf.lines() {
            if tag.starts_with('#') {
                continue;
            }
            println!("Current tags: {tag}+{}", self.options.tags.as_ref().unwrap());

            let url = base.to_owned() + "&tags=" + tag + "+" + self.options.tags.as_ref().unwrap().as_str();
            let res = reqwest::get(url).await?.text().await?;
            let doc = roxmltree::Document::parse(&res)?;
            let posts = doc.descendants().filter(|n| n.has_tag_name("post"));
            for post in posts {
                let mut nodes = post.descendants();
                let id = nodes
                    .find(|n| n.has_tag_name("id"))
                    .unwrap()
                    .text()
                    .unwrap();
                let file_url = nodes
                    .find(|n| n.has_tag_name("file_url"))
                    .unwrap()
                    .text()
                    .unwrap();

                let mut name: String;
                if self.options.quick {
                    let tags = nodes
                        .find(|n| n.has_tag_name("tags"))
                        .unwrap()
                        .text()
                        .unwrap();
                    name = tags.to_string();
                    name.truncate(TITLE_LENGTH_LIMIT);
                } else {
                    let url =
                        String::from("https://gelbooru.com/index.php?page=post&s=view&id=") + id;
                    let res = reqwest::get(url).await?.text().await?;
                    name = Self::extract_title_from_meta(&res).to_string();
                }
                let extension = file_url.split_terminator('.').next_back().unwrap();
                let path_string =
                    String::from(self.options.folder.as_ref().unwrap()) + "/" + id + " " + name.as_str() + "." + extension;
                let path = Path::new(path_string.as_str());

                if path.exists() {
                    println!("{id} already exists, skipping.");
                    continue;
                }

                print!("Downloading {id}...");
                io::stdout().flush()?;
                let img_bytes = reqwest::get(file_url).await?.bytes().await?;
                File::create(path)?.write(&img_bytes)?;
                println!("complete.");
            }
        }

        println!("Finished getting all tags.");

        Ok(())
    }

    async fn start(&self) -> Result<(), Box<dyn Error>> {
        match self.options.action {
            Action::GetPost => Self::get_posts(&self).await,
            Action::GetFavorites => Self::get_favorites(&self).await,
            Action::AddFavorites => Self::add_to_favorites(&self).await,
        }
    }
}

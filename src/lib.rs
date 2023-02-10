use std::{
    borrow::Cow,
    error::Error,
    fs::{self, File},
    io::{self, Read, Write},
    ops::Range,
    path::Path,
};

const PID_STEP_VIEW: u64 = 50;
const PID_STEP_LIST: u64 = 42;
const TITLE_LENGTH_LIMIT: usize = 100;

#[derive(Debug, Default)]
enum Action {
    #[default]
    GetPosts,
    GetFavorites,
    AddFavorites,
}

#[derive(Debug, Default)]
pub struct SessionOptions {
    action: Action,
    // Cookies and tokens
    api_key: Option<String>,
    user_id: Option<String>,
    pass_hash: Option<String>,
    fringe_benefits: Option<String>,
    // Range of pages
    start: Option<u64>,
    end: Option<u64>,
    // Queue definition
    tags: Option<String>,
    file: Option<String>,
    folder: Option<String>,
    // Methods
    use_api: bool,
    quick: bool,
}

impl SessionOptions {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn get_posts(&mut self) -> &mut Self {
        self.action = Action::GetPosts;
        self
    }
    pub fn get_favorites(&mut self) -> &mut Self {
        self.action = Action::GetFavorites;
        self
    }
    pub fn add_favorites(&mut self) -> &mut Self {
        self.action = Action::AddFavorites;
        self
    }

    pub fn user_id(&mut self, s: impl Into<String>) -> &mut Self {
        self.user_id = Some(s.into());
        self
    }
    pub fn api_key(&mut self, s: impl Into<String>) -> &mut Self {
        self.api_key = Some(s.into());
        self
    }
    pub fn pass_hash(&mut self, s: impl Into<String>) -> &mut Self {
        self.pass_hash = Some(s.into());
        self
    }
    pub fn fringe_benefits(&mut self, s: impl Into<String>) -> &mut Self {
        self.fringe_benefits = Some(s.into());
        self
    }

    pub fn begin(&mut self, n: u64) -> &mut Self {
        self.start = Some(n);
        self
    }
    pub fn end(&mut self, n: u64) -> &mut Self {
        self.end = Some(n);
        self
    }

    pub fn tags(&mut self, s: impl Into<String>) -> &mut Self {
        self.tags = Some(s.into());
        self
    }
    pub fn file(&mut self, s: impl Into<String>) -> &mut Self {
        self.file = Some(s.into());
        self
    }
    pub fn folder(&mut self, s: impl Into<String>) -> &mut Self {
        self.folder = Some(s.into());
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

    pub fn create(self) -> Session {
        Session::create(self)
    }
}

pub struct Session {
    options: SessionOptions,
}

impl Session {
    pub fn options() -> SessionOptions {
        SessionOptions::new()
    }

    fn extract_file_url(res: &str) -> &str {
        let re = regex::Regex::new(r"https://img3.gelbooru.com/(.*)\.[A-z0-9]+").unwrap();
        let mat = re.find(res).unwrap();

        &res[mat.range()]
    }

    fn extract_id_from_url(url: &str) -> &str {
        let re = regex::Regex::new("id=([0-9]+)&*").unwrap();
        let cap = re.captures(url).unwrap();
        let mat = cap.get(1).unwrap();

        &url[mat.range()]
    }

    /// Extract post title from given response,
    /// replacing all invalid characters across different platforms with `_`.
    fn extract_title(res: &str) -> Cow<str> {
        let re = regex::Regex::new("<title>(.*?)</title>").unwrap();
        let cap = re.captures(res).unwrap();
        let mat = cap.get(1).unwrap();
        let range = if mat.range().len() < TITLE_LENGTH_LIMIT {
            mat.range()
        } else {
            Range {
                start: mat.start(),
                end: mat.start() + TITLE_LENGTH_LIMIT,
            }
        };
        let re = regex::Regex::new("[/\\?%*:|\"<>]").unwrap();

        re.replace_all(&res[range], "_")
    }

    async fn new_client_webdriver(
        &self,
    ) -> Result<fantoccini::Client, fantoccini::error::CmdError> {
        let c = fantoccini::ClientBuilder::native()
            .connect("http://localhost:4444")
            .await
            .expect("failed to connect to WebDriver");

        print!("Entering home page...");
        io::stdout().flush().expect("cannot flush stdout");

        c.goto("https://gelbooru.com/index.php").await?;

        println!("adding cookies...");

        if let Some(user_id) = self.options.user_id.as_deref() {
            let cookie = fantoccini::cookies::Cookie::new("user_id", user_id.to_owned());
            c.add_cookie(cookie).await?;
        }
        if let Some(pass_hash) = self.options.pass_hash.as_deref() {
            let cookie = fantoccini::cookies::Cookie::new("pass_hash", pass_hash.to_owned());
            c.add_cookie(cookie).await?;
        }
        if let Some(fringe_benefits) = self.options.fringe_benefits.as_deref() {
            let cookie = fantoccini::cookies::Cookie::new("fringeBenefits", fringe_benefits.to_owned());
            c.add_cookie(cookie).await?;
        }

        Ok(c)
    }

    fn new_client_http(&self) -> Result<reqwest::Client, reqwest::Error> {
        let cookie = format!(
            "user_id={}; pass_hash={}; fringeBenefits={}",
            self.options.user_id.as_deref().unwrap_or_default(),
            self.options.pass_hash.as_deref().unwrap_or_default(),
            self.options.fringe_benefits.as_deref().unwrap_or_default()
        );
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert(
            reqwest::header::COOKIE,
            reqwest::header::HeaderValue::from_str(&cookie)
                .expect("invalid characters found generating cookie"),
        );

        reqwest::Client::builder().default_headers(headers).build()
    }

    async fn add_to_favorites(&self) -> Result<(), Box<dyn Error>> {
        println!("Start adding urls to favorites...");

        let client = Self::new_client_webdriver(&self).await?;

        let mut buf = String::new();
        if let Some(f) = &self.options.file {
            File::open(f)?.read_to_string(&mut buf)?;
        };
        for url in buf.lines() {
            print!("Entering {url} ...");
            io::stdout().flush().expect("cannot flush stdout");

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

        client.close().await?;

        Ok(())
    }

    async fn get_favorites(&self) -> Result<(), Box<dyn Error>> {
        println!("Start getting favorites...");

        let client = Self::new_client_webdriver(&self).await?;

        let base = format!(
            "https://gelbooru.com/index.php?page=favorites&s=view&id={}",
            self.options.user_id.as_deref().unwrap_or_default()
        );

        let range = Range {
            start: self.options.start.unwrap_or(1),
            end: self.options.end.unwrap_or(u64::MAX),
        };
        for page in range {
            print!("Entering favorites, page {page}...",);
            io::stdout().flush().expect("cannot flush stdout");

            let pid = (page - 1) * PID_STEP_VIEW;
            let url = format!("{}&pid={}", base, pid);
            client.goto(&url).await?;

            println!("getting elements...");

            let a_s = client
                .find_all(fantoccini::Locator::Css("span.thumb a:first-child"))
                .await?;
            if a_s.is_empty() {
                println!("No elements present.");
                break;
            }

            self.get_elements_webdriver(a_s).await?;
        }

        println!("Finished getting favorites.");

        client.close().await?;

        Ok(())
    }

    async fn get_elements_webdriver(
        &self,
        a_s: Vec<fantoccini::elements::Element>,
    ) -> Result<(), Box<dyn Error>> {
        'outer: for a in a_s {
            print!("Extracting information...");
            io::stdout().flush().expect("cannot flush stdout");

            let src = a.attr("href").await?.unwrap();
            let id = Self::extract_id_from_url(&src);

            let saved = fs::read_dir(self.options.folder.as_deref().unwrap_or("."))?;
            for file in saved {
                let name = file.as_ref().unwrap().file_name();
                if name.to_str().unwrap().starts_with(id) {
                    println!("{id} already exists, skipping.");
                    continue 'outer;
                }
            }

            let client = Self::new_client_http(&self)?;
            self.download_noapi(&client, id).await?;
        }

        Ok(())
    }

    async fn get_posts(&self) -> Result<(), Box<dyn Error>> {
        println!("Start getting by tags...");

        let client = Self::new_client_http(&self)?;

        let base = if self.options.use_api {
            format!(
                "https://gelbooru.com/index.php?page=dapi&s=post&q=index&api_key={}&user_id={}",
                self.options
                    .api_key
                    .as_deref()
                    .ok_or("api_key is not specified")?,
                self.options
                    .user_id
                    .as_deref()
                    .ok_or("user_id is not specified")?,
            )
        } else {
            String::from("https://gelbooru.com/index.php?page=post&s=list")
        };

        self.get_posts_with_tags(base, client).await?;

        println!("Finished getting all tags.");

        Ok(())
    }

    async fn get_posts_with_tags(
        &self,
        base: String,
        client: reqwest::Client,
    ) -> Result<(), Box<dyn Error>> {
        if let Some(f) = self.options.file.as_deref() {
            let mut buf = String::new();
            File::open(f)?.read_to_string(&mut buf)?;

            for current_tag in buf.lines() {
                if current_tag.starts_with('#') {
                    continue;
                }

                println!(
                    "Current tags: {}+{}",
                    current_tag,
                    self.options.tags.as_deref().unwrap_or_default(),
                );

                if self.options.use_api {
                    self.get_posts_api(&base, current_tag).await?;
                } else {
                    self.get_posts_noapi(&base, current_tag, &client).await?;
                }
            }
        } else {
            if self.options.use_api {
                self.get_posts_api(&base, self.options.tags.as_deref().unwrap_or_default())
                    .await?;
            } else {
                self.get_posts_noapi(
                    &base,
                    self.options.tags.as_deref().unwrap_or_default(),
                    &client,
                )
                .await?;
            }
        }

        Ok(())
    }

    async fn get_posts_noapi(
        &self,
        base: &str,
        tag: &str,
        client: &reqwest::Client,
    ) -> Result<(), Box<dyn Error>> {
        let range = Range {
            start: self.options.start.unwrap_or(1),
            end: self.options.end.unwrap_or(u64::MAX),
        };
        for page in range {
            println!("Entering posts, page {page}...");
            io::stdout().flush().expect("cannot flush stdout");

            let pid = (page - 1) * PID_STEP_LIST;
            let list_url = format!(
                "{}&tags={}+{}&pid={}",
                base,
                tag,
                self.options.tags.as_deref().unwrap_or_default(),
                pid.to_string()
            );
            let res = client.get(list_url).send().await?.text().await?;
            let list = scraper::Html::parse_document(&res);
            let selector = scraper::Selector::parse("article.thumbnail-preview a").unwrap();
            let a_s: Vec<_> = list.select(&selector).collect();
            if a_s.is_empty() {
                println!("no elements present.");
                break;
            }

            self.get_elements_http(a_s, client).await?;
        }

        Ok(())
    }

    async fn get_elements_http(
        &self,
        a_s: Vec<scraper::ElementRef<'_>>,
        client: &reqwest::Client,
    ) -> Result<(), Box<dyn Error>> {
        'outer: for a in a_s {
            print!("Extracting information...");
            io::stdout().flush().expect("cannot flush stdout");

            let href = a.value().attr("href").unwrap();
            let id = Self::extract_id_from_url(href);

            let saved = fs::read_dir(self.options.folder.as_deref().unwrap_or("."))?;
            for file in saved {
                let name = file.as_ref().unwrap().file_name();
                if name.to_str().unwrap().starts_with(id) {
                    println!("{id} already exists, skipping.");
                    continue 'outer;
                }
            }

            self.download_noapi(client, id).await?;
        }

        Ok(())
    }

    async fn download_noapi(
        &self,
        client: &reqwest::Client,
        id: &str,
    ) -> Result<(), reqwest::Error> {
        print!("entering {id} ...");
        io::stdout().flush().expect("cannot flush stdout");

        let src = format!("https://gelbooru.com/index.php?page=post&s=view&id={id}");
        let res = client.get(&src).send().await?.text().await?;

        let file_url = Self::extract_file_url(&res);
        let title = Self::extract_title(&res);
        let folder = self.options.folder.as_deref().unwrap_or(".");
        let extention = file_url.split('.').next_back().unwrap();
        let path_string = format!("./{}/{} {}.{}", folder, id, title, extention);

        print!("downloading...");
        io::stdout().flush().expect("cannot flush stdout");

        let img_bytes = client.get(file_url).send().await?.bytes().await?;
        File::create(&path_string)
            .expect("cannot create image file")
            .write(&img_bytes)
            .expect("cannot write to image file");
        println!("complete.");
        Ok(())
    }

    async fn get_posts_api(&self, base: &str, tag: &str) -> Result<(), Box<dyn Error>> {
        let range = Range {
            start: self.options.start.unwrap_or(1),
            end: self.options.end.unwrap_or(u64::MAX),
        };
        for page in range {
            let list_url = format!(
                "{}&tags={}+{}&pid={}&limit={}",
                base,
                tag,
                self.options.tags.as_deref().unwrap_or_default(),
                page,
                PID_STEP_LIST,
            );
            let res = reqwest::get(list_url).await?.text().await?;
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
                let name = if self.options.quick {
                    let tags = nodes
                        .find(|n| n.has_tag_name("tags"))
                        .unwrap()
                        .text()
                        .unwrap();
                    let mut temp = tags.to_string();
                    temp.truncate(TITLE_LENGTH_LIMIT);
                    temp
                } else {
                    let url =
                        String::from("https://gelbooru.com/index.php?page=post&s=view&id=") + id;
                    let res = reqwest::get(url).await?.text().await?;
                    Self::extract_title(&res).to_string()
                };
                let extension = file_url.split('.').next_back().unwrap();
                let path_string = format!(
                    "./{}/{} {}.{}",
                    self.options.folder.as_deref().unwrap_or("."),
                    id,
                    name,
                    extension,
                );

                let path = Path::new(&path_string);
                if path.exists() {
                    println!("{id} already exists, skipping.");
                    continue;
                }

                print!("Downloading {id}...");
                io::stdout().flush().expect("cannot flush stdout");
                let img_bytes = reqwest::get(file_url).await?.bytes().await?;
                File::create(path)?.write(&img_bytes)?;
                println!("complete.");
            }
        }

        Ok(())
    }

    fn create(options: SessionOptions) -> Self {
        Self { options }
    }

    pub async fn start(&self) -> Result<(), Box<dyn Error>> {
        match self.options.action {
            Action::GetPosts => Self::get_posts(&self).await?,
            Action::GetFavorites => Self::get_favorites(&self).await?,
            Action::AddFavorites => Self::add_to_favorites(&self).await?,
        };

        Ok(())
    }
}

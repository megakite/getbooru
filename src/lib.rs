use std::{
    borrow::Cow,
    error::Error,
    fs::{self, File},
    io::{self, Read, Write},
    ops::{Range, RangeInclusive},
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
    api: bool,
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

    pub fn start(&mut self, n: u64) -> &mut Self {
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

    pub fn api(&mut self, b: bool) -> &mut Self {
        self.api = b;
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

    fn create(options: SessionOptions) -> Self {
        Self { options }
    }

    fn extract_file_url(res: &str) -> Result<&str, &'static str> {
        let re = regex::Regex::new(r"https://img3.gelbooru.com/(.*)\.[A-z0-9]+").unwrap();
        let mat = re.find(res).ok_or("cannot find file url")?;

        Ok(&res[mat.range()])
    }

    fn extract_id_from_url(url: &str) -> Result<&str, &'static str> {
        let re = regex::Regex::new("id=([0-9]+)&*").unwrap();
        let cap = re.captures(url).ok_or("cannot find id")?;
        let mat = cap.get(1).ok_or("cannot get id")?;

        Ok(&url[mat.range()])
    }

    /// Extract post title from given response,
    /// replacing all invalid characters across different platforms with `_`.
    fn extract_title(res: &str) -> Result<Cow<str>, &'static str> {
        let re = regex::Regex::new("<title>(.*?)</title>").unwrap();
        let cap = re.captures(res).ok_or("cannot find title")?;
        let mat = cap.get(1).ok_or("cannot get title")?;
        let range = if mat.range().len() < TITLE_LENGTH_LIMIT {
            mat.range()
        } else {
            Range {
                start: mat.start(),
                end: mat.start() + TITLE_LENGTH_LIMIT,
            }
        };
        let re = regex::Regex::new("[/\\?%*:|\"<>]").unwrap();

        Ok(re.replace_all(&res[range], "_"))
    }

    async fn new_client_webdriver(
        &self,
    ) -> Result<fantoccini::Client, fantoccini::error::CmdError> {
        let c = fantoccini::ClientBuilder::native()
            .connect("http://localhost:4444")
            .await
            .expect("a WebDriver should be running on port 4444");

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
            let cookie =
                fantoccini::cookies::Cookie::new("fringeBenefits", fringe_benefits.to_owned());
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
                .expect("invalid characters found when generating Cookie header"),
        );

        reqwest::Client::builder().default_headers(headers).build()
    }

    async fn add_to_favorites(&self) -> Result<(), Box<dyn Error>> {
        println!("Start adding urls to favorites...");

        let client = self.new_client_webdriver().await?;

        let mut buf = String::new();
        if let Some(f) = &self.options.file {
            File::open(f)?.read_to_string(&mut buf)?;
        };
        for url in buf.lines() {
            print!("Entering {} ...", url);
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

        let client = self.new_client_webdriver().await?;

        let base = format!(
            "https://gelbooru.com/index.php?page=favorites&s=view&id={}",
            self.options.user_id.as_deref().unwrap_or_default()
        );

        let range = RangeInclusive::new(
            self.options.start.unwrap_or(1),
            self.options.end.unwrap_or(u64::MAX),
        );
        for page in range {
            print!("Entering favorites, page {}...", page);
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

            let src = a
                .attr("href")
                .await?
                .ok_or("cannot find href in thumbnail element")?;
            let id = Self::extract_id_from_url(&src)?;

            let saved = fs::read_dir(self.options.folder.as_deref().unwrap_or("."))?;
            for file in saved {
                let name = file?.file_name();
                if name
                    .to_str()
                    .ok_or("invalid character in saved file names")?
                    .starts_with(id)
                {
                    println!("{id} already exists, skipping.");
                    continue 'outer;
                }
            }

            let client = self.new_client_http()?;
            self.download(&client, id).await?;
        }

        Ok(())
    }

    async fn get_posts(&self) -> Result<(), Box<dyn Error>> {
        println!("Start getting posts...");

        let client = self.new_client_http()?;

        let base = if self.options.api {
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

        self.get_posts_with_tags(&base, &client).await?;

        println!("Finished getting all tags.");

        Ok(())
    }

    async fn get_posts_with_tags(
        &self,
        base: &str,
        client: &reqwest::Client,
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

                if self.options.api {
                    self.get_posts_api(base, current_tag).await?;
                } else {
                    self.get_posts_noapi(base, current_tag, client).await?;
                }
            }
        } else {
            if self.options.api {
                self.get_posts_api(base, self.options.tags.as_deref().unwrap_or_default())
                    .await?;
            } else {
                self.get_posts_noapi(
                    base,
                    self.options.tags.as_deref().unwrap_or_default(),
                    client,
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
        let range = RangeInclusive::new(
            self.options.start.unwrap_or(1),
            self.options.end.unwrap_or(u64::MAX),
        );
        for page in range {
            println!("Entering posts, page {}...", page);
            io::stdout().flush().expect("cannot flush stdout");

            let pid = (page - 1) * PID_STEP_LIST;
            let list_url = format!(
                "{}&tags={}+{}&pid={}",
                base,
                tag,
                self.options.tags.as_deref().unwrap_or_default(),
                pid
            );
            let res = client.get(list_url).send().await?.text().await?;
            let list = scraper::Html::parse_document(&res);
            let selector = scraper::Selector::parse("article.thumbnail-preview a")?;
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

            let href = a
                .value()
                .attr("href")
                .ok_or("cannot find href in thumbnail element")?;
            let id = Self::extract_id_from_url(href)?;

            let saved = fs::read_dir(self.options.folder.as_deref().unwrap_or("."))?;
            for file in saved {
                let name = file?.file_name();
                if name
                    .to_str()
                    .ok_or("invalid character in saved file names")?
                    .starts_with(id)
                {
                    println!("{id} already exists, skipping.");
                    continue 'outer;
                }
            }

            self.download(client, id).await?;
        }

        Ok(())
    }

    async fn download(&self, client: &reqwest::Client, id: &str) -> Result<(), Box<dyn Error>> {
        print!("entering {id} ...");
        io::stdout().flush().expect("cannot flush stdout");

        let src = format!("https://gelbooru.com/index.php?page=post&s=view&id={id}");
        let res = client.get(&src).send().await?.text().await?;

        let file_url = Self::extract_file_url(&res)?;
        let title = Self::extract_title(&res)?;
        let folder = self.options.folder.as_deref().unwrap_or(".");
        let extention = file_url
            .split('.')
            .next_back()
            .ok_or("source file has no extension")?;
        let path_string = format!("./{}/{} {}.{}", folder, id, title, extention);

        print!("downloading...");
        io::stdout().flush().expect("cannot flush stdout");

        let img_bytes = client.get(file_url).send().await?.bytes().await?;
        File::create(path_string)?.write_all(&img_bytes)?;
        println!("complete.");
        Ok(())
    }

    async fn get_posts_api(&self, base: &str, tag: &str) -> Result<(), Box<dyn Error>> {
        let range = RangeInclusive::new(
            self.options.start.unwrap_or(1),
            self.options.end.unwrap_or(u64::MAX),
        );
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
                self.get_post_by_node(post).await?;
            }
        }

        Ok(())
    }

    async fn get_post_by_node(&self, post: roxmltree::Node<'_, '_>) -> Result<(), Box<dyn Error>> {
        let mut nodes = post.descendants();

        let id = nodes
            .find(|n| n.has_tag_name("id"))
            .ok_or("cannot find XML tag <id>")?
            .text()
            .ok_or("no text in XML tag <id>")?;
        let file_url = nodes
            .find(|n| n.has_tag_name("file_url"))
            .ok_or("cannot find XML tag <file_url>")?
            .text()
            .ok_or("no text in XML tag <file_url>")?;
        let name = if self.options.quick {
            let tags = nodes
                .find(|n| n.has_tag_name("tags"))
                .ok_or("cannot find XML tag <tags>")?
                .text()
                .ok_or("no text in XML tag <tags>")?;
            if tags.len() < TITLE_LENGTH_LIMIT {
                String::from(tags)
            } else {
                String::from(&tags[0..TITLE_LENGTH_LIMIT])
            }
        } else {
            let url = String::from("https://gelbooru.com/index.php?page=post&s=view&id=") + id;
            let res = reqwest::get(url).await?.text().await?;
            Self::extract_title(&res)?.to_string()
        };
        let extension = file_url
            .split('.')
            .next_back()
            .ok_or("source file has no extension")?;
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
            return Ok(());
        }

        let client = self.new_client_http()?;
        self.download(&client, id).await?;

        Ok(())
    }

    pub async fn start(&self) -> Result<(), Box<dyn Error>> {
        match self.options.action {
            Action::GetPosts => self.get_posts().await?,
            Action::GetFavorites => self.get_favorites().await?,
            Action::AddFavorites => self.add_to_favorites().await?,
        };

        Ok(())
    }
}

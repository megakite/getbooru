use std::{
    error::Error,
    fs::{self, File},
    io::{self, Read, Write},
    ops::Range,
    path::Path,
};

const PID_STEP_VIEW: u64 = 50;
const PID_STEP_LIST: u64 = 42;
const TITLE_LENGTH_LIMIT: usize = 100;

#[derive(Clone, Debug, Default)]
enum Action {
    #[default]
    GetPosts,
    GetFavorites,
    AddFavorites,
}

#[derive(Clone, Debug, Default)]
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

    pub fn begin(&mut self, n: u64) -> &mut Self {
        self.start = Some(n);
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

    pub fn create(&self) -> Session {
        Session::create(&self)
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
        let begin = res.find("content=\"https://").unwrap() + "content=\"".len();
        let end = res.find("\" />\n\t\t<meta name=\"twitter:card\"").unwrap();

        &res[begin..end]
    }

    fn extract_title(res: &str) -> std::borrow::Cow<str> {
        let begin = res.find("<title>").unwrap() + "<title>".len();
        let mut end = res.find("</title>").unwrap();
        if end - begin > TITLE_LENGTH_LIMIT {
            end = begin + TITLE_LENGTH_LIMIT;
        }
        let re = regex::Regex::new("[/\\?%*:|\"<>]").unwrap();
        let title = re.replace_all(&res[begin..end], "_");

        title
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

        if let Some(ui) = &self.options.user_id {
            c.add_cookie(fantoccini::cookies::Cookie::new("user_id", ui.to_owned()))
                .await?;
        }
        if let Some(ph) = &self.options.pass_hash {
            c.add_cookie(fantoccini::cookies::Cookie::new("pass_hash", ph.to_owned()))
                .await?;
        }
        if let Some(fb) = &self.options.fringe_benefits {
            c.add_cookie(fantoccini::cookies::Cookie::new(
                "fringeBenefits",
                fb.to_owned(),
            ))
            .await?;
        }

        Ok(c)
    }

    fn new_client_http(&self) -> Result<reqwest::Client, reqwest::Error> {
        let mut headers = reqwest::header::HeaderMap::new();
        let cookie = format!(
            "user_id={}; pass_hash={}; fringeBenefits={}",
            self.options.user_id.as_ref().unwrap_or(&String::new()),
            self.options.pass_hash.as_ref().unwrap_or(&String::new()),
            self.options
                .fringe_benefits
                .as_ref()
                .unwrap_or(&String::new()),
        );
        headers.insert(
            reqwest::header::COOKIE,
            reqwest::header::HeaderValue::from_str(&cookie).unwrap(),
        );

        reqwest::Client::builder().default_headers(headers).build()
    }

    async fn get_view(
        &self,
        client: &fantoccini::Client,
        token: &str,
    ) -> Result<(), Box<dyn Error>> {
        let page_type = "post";

        let base = format!(
            "https://gelbooru.com/index.php?page={}&s=view&{}",
            page_type, token
        );
        let downloaded: Vec<_> = fs::read_dir("saved/")?.collect();

        let range = Range {
            start: self.options.start.unwrap_or(1),
            end: self.options.end.unwrap_or(u64::MAX),
        };
        for page in range {
            print!("Entering {page_type}, page {page}...",);
            io::stdout().flush().expect("cannot flush stdout");

            let pid = (page - 1) * PID_STEP_VIEW;
            let url = format!("{}&pid={}", base, pid);
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
                io::stdout().flush().expect("cannot flush stdout");

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

                let client = Self::new_client_http(&self)?;
                self.download_noapi(&client, id).await?;
            }
        }

        Ok(())
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
        Ok(())
    }

    async fn get_favorites(&self) -> Result<(), Box<dyn Error>> {
        println!("Start getting favorites...");

        let client = Self::new_client_webdriver(&self).await?;

        let token = format!(
            "id={}",
            self.options
                .user_id
                .as_ref()
                .expect("user_id is not specified")
        );
        Self::get_view(&self, &client, &token).await?;

        println!("Finished getting favorites.");

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
                    .as_ref()
                    .expect("api_key is not specified"),
                self.options
                    .user_id
                    .as_ref()
                    .expect("user_id is not specified"),
            )
        } else {
            String::from("https://gelbooru.com/index.php?page=post&s=list")
        };

        if let Some(f) = &self.options.file {
            let mut buf = String::new();
            File::open(f)?.read_to_string(&mut buf)?;

            self.get_posts_by_buf(buf, base, client).await?;
        } else {
            self.get_posts_by_tag(base, client).await?;
        }

        println!("Finished getting all tags.");

        Ok(())
    }

    async fn get_posts_by_tag(
        &self,
        base: String,
        client: reqwest::Client,
    ) -> Result<(), Box<dyn Error>> {
        if self.options.use_api {
            self.get_posts_api(&base, self.options.tags.as_ref().unwrap_or(&String::new()))
                .await?;
        } else {
            self.get_posts_noapi(
                &base,
                self.options.tags.as_ref().unwrap_or(&String::new()),
                &client,
            )
            .await?;
        }

        Ok(())
    }

    async fn get_posts_by_buf(
        &self,
        buf: String,
        base: String,
        client: reqwest::Client,
    ) -> Result<(), Box<dyn Error>> {
        for tag in buf.lines() {
            if tag.starts_with('#') {
                continue;
            }

            println!(
                "Current tags: {tag}+{}",
                self.options.tags.as_ref().unwrap_or(&String::new()),
            );

            if self.options.use_api {
                self.get_posts_api(&base, tag).await?;
            } else {
                self.get_posts_noapi(&base, tag, &client).await?;
            }
        }

        Ok(())
    }

    async fn get_posts_noapi(
        &self,
        base: &String,
        tag: &str,
        client: &reqwest::Client,
    ) -> Result<(), Box<dyn Error>> {
        let range = Range {
            start: self.options.start.unwrap_or(1),
            end: self.options.end.unwrap_or(u64::MAX),
        };
        for page in range {
            print!("Entering posts, page {page}...");
            std::io::stdout().flush().expect("cannot flush stdout");
            let pid = (page - 1) * PID_STEP_LIST;
            let list_url = format!(
                "{}&tags={}+{}&pid={}",
                base,
                tag,
                self.options.tags.as_ref().unwrap_or(&String::new()),
                pid.to_string()
            );

            println!("getting elements...");

            let res = client.get(list_url).send().await?.text().await?;
            let list = scraper::Html::parse_document(&res);
            let selector = scraper::Selector::parse("article.thumbnail-preview a").unwrap();
            let a_s: Vec<_> = list.select(&selector).collect();
            if a_s.is_empty() {
                println!("No elements present.");
                break;
            }

            'outer: for a in a_s {
                print!("Extracting information...");
                io::stdout().flush().expect("cannot flush stdout");

                let src = a.value().attr("href").unwrap().to_string();
                let start = src.find("&id=").unwrap() + "&id=".len();
                let end = src.find("&tags=").unwrap_or(src.len() + 1);
                let id = &src[start..end];

                let downloaded: Vec<_> =
                    fs::read_dir(self.options.folder.as_ref().unwrap_or(&".".to_string()))?
                        .collect();
                for file in &downloaded {
                    let name = file.as_ref().unwrap().file_name();
                    if name.to_str().unwrap().starts_with(id) {
                        println!("{id} already exists, skipping.");
                        continue 'outer;
                    }
                }

                self.download_noapi(client, id).await?;
            }
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
        let default_folder = String::from('.');
        let title = Self::extract_title(&res);
        let folder = self.options.folder.as_ref().unwrap_or(&default_folder);
        let extention = Path::new(file_url).extension().unwrap().to_str().unwrap();
        let path_string = format!("{}/{} {}.{}", folder, id, title, extention,);

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

    async fn get_posts_api(&self, base: &String, tag: &str) -> Result<(), Box<dyn Error>> {
        let range = Range {
            start: self.options.start.unwrap_or(1),
            end: self.options.end.unwrap_or(u64::MAX),
        };
        for page in range {
            let list_url = format!(
                "{}&tags={}+{}&pid={}&limit={}",
                base,
                tag,
                self.options.tags.as_ref().unwrap_or(&String::new()),
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
                let extension = file_url.split_terminator('.').next_back().unwrap();
                let path_string = format!(
                    "{}/{} {}.{}",
                    self.options.folder.as_ref().unwrap_or(&String::from('.')),
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

    fn create(opts: &SessionOptions) -> Self {
        Self {
            options: opts.clone(),
        }
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

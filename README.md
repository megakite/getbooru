# getbooru

Simple scraper for certain anime gallery.\
某图站的爬虫。

## Usage 用法

(Optional) Firstly, create a .env file with the following content:\
（可选）首先在当前目录下创建一个 .env 文件：

```shell
# Your Gelbooru API_KEY and USER_ID. Get them from account options.
# You can also leave them blank, but the functionality will be limited.
api_key=ffffffff00000000ffffffff00000000ffffffff00000000ffffffff00000000
user_id=2147483647
# If you haven't donated to Gelbooru, you probably want to fill out the next
# terms in order to perform normally using 'noapi' mode, after the daily limit
# of requests using site API has been reached.
# Password hash of your account. This can be found in your site cookies.
pass_hash=ffff0000ffff0000ffff0000ffff0000ffff0000
# Display all contents ("yup"), or not (leave blank).
# This doesn't require an account.
fringeBenefits=yup
```

Then from the shell:\
然后按如下方式执行命令：

```shell
getbooru get favorites # (WebDriver needed) Get all of your favorites into current directory
getbooru add favorites by urls.txt # (WebDriver needed) Add urls in links.txt to your favorites
getbooru get posts from 6 to 9 api # Get posts in page 6-9, using API
getbooru get posts with 1boy into dir # Get posts with tag '1boy' into ./dir/
```

Note: `api` can be combined with `quick`, which speeds up the progress but alternates file names.\
注：`api` 选项可与 `quick` 选项合用以提升速度，但文件名会发生变化。

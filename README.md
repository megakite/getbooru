# getbooru

Simple scraper for certain anime gallery.

## Usage

```shell
getbooru get favorites # (WebDriver needed) Get all of your favorites into current directory
getbooru add favorites by urls.txt # (WebDriver needed) Add urls in links.txt to your favorites
getbooru get posts from 6 to 9 api # Get posts in page 6-9, using API
getbooru get posts with 1boy into dir # Get posts with tag '1boy' into ./dir/
```

Note: `api` can be combined with `quick`, which speeds up the progress but alternates file names.
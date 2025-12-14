# 🥘 Couscous Crawler

![Rust](https://img.shields.io/badge/Rust-000000?style=flat&logo=rust&logoColor=white)
![Tokio](https://img.shields.io/badge/Tokio-000000?style=flat&logo=rust&logoColor=white)
![SQLite](https://img.shields.io/badge/SQLite-003B57?style=flat&logo=sqlite&logoColor=white)

A fast asynchronous web crawler written in Rust that extracts email addresses using regex and stores them in SQLite.

## Installation

```bash
git clone https://github.com/Arthur-91140/couscous-Crawler.git
cd couscous-Crawler
cargo build --release
```

## Usage

```bash
# Basic crawl
couscous-crawler https://example.com

# With depth limit
couscous-crawler https://example.com --depth 3

# Stay on same domain
couscous-crawler https://example.com --stay-on-domain

# All options
couscous-crawler https://example.com -d 2 -s -w 15 -v --db results.db
```

### Options

| Option | Description | Default |
|--------|-------------|---------|
| `-d, --depth` | Max crawl depth (0 = unlimited) | 0 |
| `-s, --stay-on-domain` | Only crawl same domain | false |
| `-w, --workers` | Async workers count | 10 |
| `--db` | Database path | emails.db |
| `-v, --verbose` | Verbose output | false |

## Credits

| Crate | Description | License |
|-------|-------------|---------|
| [tokio](https://crates.io/crates/tokio) | Async runtime | MIT |
| [reqwest](https://crates.io/crates/reqwest) | HTTP client | MIT/Apache-2.0 |
| [scraper](https://crates.io/crates/scraper) | HTML parsing | MIT |
| [rusqlite](https://crates.io/crates/rusqlite) | SQLite bindings | MIT |
| [clap](https://crates.io/crates/clap) | CLI argument parsing | MIT/Apache-2.0 |
| [regex](https://crates.io/crates/regex) | Regular expressions | MIT/Apache-2.0 |
| [url](https://crates.io/crates/url) | URL parsing | MIT/Apache-2.0 |
| [lazy_static](https://crates.io/crates/lazy_static) | Lazy statics | MIT/Apache-2.0 |
| [yolo-face](https://github.com/YapaLab/yolo-face.git) | yolo face detection | GNU GPL V3.0 |
| [ultralytics](https://github.com/ultralytics/ultralytics.git) | ultralytics | AGPL V3.0 |
| [opencv-python](https://pypi.org/project/opencv-python/) | Computer vision & webcam | Apache-2.0 |
| [pillow](https://pypi.org/project/pillow/) | Image processing for Tkinter | HPND |
| [numpy](https://pypi.org/project/numpy/) | Array computing | BSD-3-Clause |
| [scipy](https://pypi.org/project/scipy/) | Scientific computing (splines) | BSD-3-Clause |
| [lap](https://pypi.org/project/lap/) | Linear assignment for tracking | BSD-2-Clause |

## License

MIT

---

Coded with love by [@Arthur-91140](https://github.com/Arthur-91140)
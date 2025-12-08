# Couscous Crawler

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

## License

MIT

---

Coded with love by [@Arthur-91140](https://github.com/Arthur-91140)
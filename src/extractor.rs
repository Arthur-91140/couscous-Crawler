use regex::Regex;
use scraper::{Html, Selector};
use url::Url;
use std::collections::HashSet;

lazy_static::lazy_static! {
    static ref EMAIL_REGEX: Regex = Regex::new(
        r"[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}"
    ).unwrap();
}

/// Extract all email addresses from HTML content
pub fn extract_emails(html: &str) -> Vec<String> {
    let mut emails: HashSet<String> = HashSet::new();
    
    for capture in EMAIL_REGEX.find_iter(html) {
        let email = capture.as_str().to_lowercase();
        // Filter out common false positives
        if !is_false_positive(&email) {
            emails.insert(email);
        }
    }
    
    emails.into_iter().collect()
}

/// Check if an email-like string is a false positive
fn is_false_positive(email: &str) -> bool {
    // Filter out image files and common false patterns
    let false_extensions = [".png", ".jpg", ".jpeg", ".gif", ".svg", ".webp", ".ico"];
    for ext in false_extensions {
        if email.ends_with(ext) {
            return true;
        }
    }
    
    // Filter out very short or suspicious patterns
    if email.len() < 5 {
        return true;
    }
    
    false
}

/// Extract all links from HTML content
pub fn extract_links(html: &str, base_url: &Url) -> Vec<Url> {
    let document = Html::parse_document(html);
    let selector = Selector::parse("a[href]").unwrap();
    let mut links: HashSet<Url> = HashSet::new();
    
    for element in document.select(&selector) {
        if let Some(href) = element.value().attr("href") {
            // Skip javascript:, mailto:, tel:, etc.
            if href.starts_with("javascript:") 
                || href.starts_with("mailto:") 
                || href.starts_with("tel:")
                || href.starts_with("#")
                || href.is_empty() 
            {
                continue;
            }
            
            // Try to resolve the URL
            if let Ok(resolved) = base_url.join(href) {
                // Only keep http/https links
                if resolved.scheme() == "http" || resolved.scheme() == "https" {
                    // Remove fragment
                    let mut clean_url = resolved.clone();
                    clean_url.set_fragment(None);
                    links.insert(clean_url);
                }
            }
        }
    }
    
    links.into_iter().collect()
}

/// Check if a URL belongs to the same domain as the base
pub fn is_same_domain(url: &Url, base_domain: &str) -> bool {
    url.host_str()
        .map(|host| host == base_domain || host.ends_with(&format!(".{}", base_domain)))
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_emails() {
        let html = r#"
            <html>
                <body>
                    Contact us at test@example.com or info@company.org
                    Image: background@2x.png
                </body>
            </html>
        "#;
        
        let emails = extract_emails(html);
        assert!(emails.contains(&"test@example.com".to_string()));
        assert!(emails.contains(&"info@company.org".to_string()));
        assert!(!emails.iter().any(|e| e.contains(".png")));
    }

    #[test]
    fn test_extract_links() {
        let html = r#"
            <html>
                <body>
                    <a href="/page1">Page 1</a>
                    <a href="https://example.com/page2">Page 2</a>
                    <a href="mailto:test@test.com">Email</a>
                </body>
            </html>
        "#;
        
        let base = Url::parse("https://example.com").unwrap();
        let links = extract_links(html, &base);
        
        assert!(links.iter().any(|u| u.path() == "/page1"));
        assert!(links.iter().any(|u| u.path() == "/page2"));
        assert!(!links.iter().any(|u| u.scheme() == "mailto"));
    }
}

use regex::Regex;
use scraper::{Html, Selector};
use url::Url;
use std::collections::HashSet;

lazy_static::lazy_static! {
    static ref EMAIL_REGEX: Regex = Regex::new(
        r"[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}"
    ).unwrap();

    // French phone number patterns
    // Supports: +33 01 02 03 04 05, 0102030405, +330102030405, 01 02 03 04 05, etc.
    static ref PHONE_REGEX: Regex = Regex::new(
        r"(?x)
        (?:
            (?:\+33\s?|0)           # +33 (with optional space) or 0 prefix
            [1-9]                    # First digit after prefix (not 0)
            (?:[\s.\-]?\d{2}){4}     # 4 groups of 2 digits with optional separators
        )
        "
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

/// Extract all French phone numbers from HTML content
pub fn extract_phones(html: &str) -> Vec<String> {
    let mut phones: HashSet<String> = HashSet::new();
    
    for capture in PHONE_REGEX.find_iter(html) {
        let phone = capture.as_str();
        // Normalize the phone number
        let normalized = normalize_phone(phone);
        if !normalized.is_empty() {
            phones.insert(normalized);
        }
    }
    
    phones.into_iter().collect()
}

/// Normalize a French phone number to a standard format
fn normalize_phone(phone: &str) -> String {
    // Remove all non-digit characters except the leading +
    let digits: String = phone.chars()
        .filter(|c| c.is_ascii_digit() || *c == '+')
        .collect();
    
    // Convert +33 to 0 for consistency
    if digits.starts_with("+33") {
        format!("0{}", &digits[3..])
    } else if digits.starts_with("33") && digits.len() == 11 {
        format!("0{}", &digits[2..])
    } else {
        digits
    }
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

    #[test]
    fn test_extract_phones() {
        let html = r#"
            <html>
                <body>
                    Appelez-nous au 01 02 03 04 05
                    Ou au +33 6 12 34 56 78
                    Fax: 0102030405
                    Mobile: +33612345678
                    Autre: 01.02.03.04.05
                    International: +33 1 02 03 04 05
                </body>
            </html>
        "#;
        
        let phones = extract_phones(html);
        assert!(phones.contains(&"0102030405".to_string()));
        assert!(phones.contains(&"0612345678".to_string()));
    }

    #[test]
    fn test_normalize_phone() {
        assert_eq!(normalize_phone("+33 1 02 03 04 05"), "0102030405");
        assert_eq!(normalize_phone("01 02 03 04 05"), "0102030405");
        assert_eq!(normalize_phone("0102030405"), "0102030405");
        assert_eq!(normalize_phone("+33612345678"), "0612345678");
        assert_eq!(normalize_phone("01.02.03.04.05"), "0102030405");
        assert_eq!(normalize_phone("01-02-03-04-05"), "0102030405");
    }
}

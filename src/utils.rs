use url::Url;

pub fn get_domain(url: &str) -> Option<String> {
    Url::parse(url)
        .ok()
        .and_then(|url| url.host_str().map(|s| s.to_string()))
        .map(|domain| {
            if domain.starts_with("www.") && domain.matches('.').count() > 1 {
                domain[4..].to_string()
            } else {
                domain
            }
        })
}

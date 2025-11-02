use regex::Regex;

pub fn parse_scheme(url: &str) -> (&str, &str) {
    let scheme_regex = Regex::new(r#"\A(\w+):\/\/(.+)\z"#).unwrap();
    let (scheme, body) = match scheme_regex.captures_iter(url).map(|c| c.extract()).next() {
        Some((_, [scheme, body])) => (scheme, body),
        None => ("file", url),
    };
    (scheme, body)
}

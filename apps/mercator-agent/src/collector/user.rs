pub fn collect_current_user() -> Option<String> {
    let username = std::env::var("USERNAME")
        .or_else(|_| std::env::var("USER"))
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())?;

    if let Ok(domain) = std::env::var("USERDOMAIN") {
        let domain = domain.trim();
        if !domain.is_empty() {
            return Some(format!("{domain}\\{username}"));
        }
    }

    Some(username)
}

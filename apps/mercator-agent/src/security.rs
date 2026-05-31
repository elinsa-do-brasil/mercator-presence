use thiserror::Error;

#[derive(Debug, Error)]
pub enum SecurityError {
    #[error("serverUrl must use HTTPS")]
    InsecureServerUrl,
    #[error("serverUrl cannot be empty")]
    EmptyServerUrl,
}

pub fn validate_server_url(server_url: &str) -> Result<(), SecurityError> {
    let trimmed = server_url.trim();
    if trimmed.is_empty() {
        return Err(SecurityError::EmptyServerUrl);
    }
    if !trimmed.starts_with("https://") {
        return Err(SecurityError::InsecureServerUrl);
    }
    Ok(())
}

pub fn mask_secret(secret: &str) -> String {
    let trimmed = secret.trim();
    if trimmed.is_empty() {
        return String::new();
    }

    let chars = trimmed.chars().collect::<Vec<_>>();
    if chars.len() <= 8 {
        return "****".to_string();
    }

    let prefix = chars.iter().take(4).collect::<String>();
    let suffix = chars
        .iter()
        .rev()
        .take(4)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect::<String>();
    format!("{prefix}...{suffix}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mask_secret_should_hide_short_values() {
        assert_eq!(mask_secret("abc123"), "****");
    }

    #[test]
    fn mask_secret_should_keep_edges_only() {
        assert_eq!(mask_secret("abcd1234wxyz"), "abcd...wxyz");
    }
}

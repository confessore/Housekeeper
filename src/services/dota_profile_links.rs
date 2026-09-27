pub fn opendota_url(account_id: i64) -> String {
    format!("https://www.opendota.com/players/{account_id}")
}

pub fn dotabuff_url(account_id: i64) -> String {
    format!("https://www.dotabuff.com/players/{account_id}")
}

pub fn stratz_url(account_id: i64) -> String {
    format!("https://stratz.com/players/{account_id}")
}

pub fn markdown_line(account_id: i64) -> String {
    format!(
        "[OpenDota]({}) · [Dotabuff]({}) · [Stratz]({})",
        opendota_url(account_id),
        dotabuff_url(account_id),
        stratz_url(account_id)
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_profile_links() {
        assert_eq!(
            markdown_line(123),
            "[OpenDota](https://www.opendota.com/players/123) · [Dotabuff](https://www.dotabuff.com/players/123) · [Stratz](https://stratz.com/players/123)"
        );
    }
}

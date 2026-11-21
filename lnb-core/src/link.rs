//! リンク URL の整形。

use url::Url;

/// URL のクエリパラメーターから `utm_source` を取り除く。
/// クエリが空になった場合は `?` も取り除く。
/// URL としてパースできない場合はそのまま返す。
pub fn strip_utm_source(url: &str) -> String {
    let Ok(mut parsed_url) = Url::parse(url) else {
        return url.to_string();
    };
    if parsed_url.query().is_none() {
        return parsed_url.to_string();
    }

    let remaining_pairs: Vec<_> = parsed_url
        .query_pairs()
        .filter(|(key, _)| key != "utm_source")
        .map(|(key, value)| (key.into_owned(), value.into_owned()))
        .collect();
    if remaining_pairs.is_empty() {
        parsed_url.set_query(None);
    } else {
        parsed_url.query_pairs_mut().clear().extend_pairs(remaining_pairs);
    }

    parsed_url.to_string()
}

#[cfg(test)]
mod tests {
    use super::strip_utm_source;

    #[test]
    fn strips_utm_source_only() {
        assert_eq!(
            strip_utm_source("https://example.com/path?utm_source=foo&bar=baz"),
            "https://example.com/path?bar=baz"
        );
    }

    #[test]
    fn removes_question_mark_when_query_becomes_empty() {
        assert_eq!(
            strip_utm_source("https://example.com/path?utm_source=foo"),
            "https://example.com/path"
        );
        assert_eq!(
            strip_utm_source("https://example.com/path?utm_source=foo#frag"),
            "https://example.com/path#frag"
        );
    }

    #[test]
    fn keeps_url_without_query() {
        assert_eq!(strip_utm_source("https://example.com/path"), "https://example.com/path");
    }
}

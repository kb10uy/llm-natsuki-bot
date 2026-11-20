use std::{
    fmt::{Result as FmtResult, Write},
    sync::LazyLock,
};

use html2md::parse_html;
use markdown::{Constructs, ParseOptions, mdast::Node};
use regex::Regex;
use url::Url;

static RE_HEAD_MENTION: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"^\s*(\[@.+?\]\(.+?\)\s*)+"#).expect("invalid regex"));

// https://github.com/mastodon/mastodon/blob/7d2dda97b3610747867861c0c7155f3e2ad94a47/app/models/account.rb#L73-L74
static RE_ESCAPING_MENTION: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"(^|[^=/[:word:]])@([[:word:]]+)"#).expect("invalid regex"));

pub fn escape_mention_html_from_mastodon(mention_html: &str) -> String {
    let content_markdown = parse_html(mention_html);
    RE_HEAD_MENTION.replace_all(&content_markdown, "").to_string()
}

pub fn process_markdown_for_mastodon(markdown_text: &str) -> (String, Vec<String>) {
    let markdown_ast = markdown::to_mdast(
        markdown_text,
        &ParseOptions {
            constructs: Constructs {
                math_text: true,
                math_flow: true,
                ..Default::default()
            },
            ..Default::default()
        },
    )
    .expect("normal markdown parse never fails");
    let Node::Root(root) = markdown_ast else {
        unreachable!("root must be Node::Root");
    };

    let mut filtered_text = String::new();
    let mut math_formulae = Vec::new();
    walk_mastodon(&mut filtered_text, &mut math_formulae, root.children).expect("must succeed");
    (filtered_text, math_formulae)
}

fn walk_mastodon(writer: &mut impl Write, math_formulae: &mut Vec<String>, children: Vec<Node>) -> FmtResult {
    for child in children {
        match child {
            Node::Root(root) => walk_mastodon(writer, math_formulae, root.children)?,

            Node::Text(text) => write_text_element(writer, &text.value)?,
            Node::Break(_) => writeln!(writer)?,
            Node::Strong(strong) => walk_mastodon(writer, math_formulae, strong.children)?,
            Node::Emphasis(emphasis) => walk_mastodon(writer, math_formulae, emphasis.children)?,
            Node::Delete(delete) => walk_mastodon(writer, math_formulae, delete.children)?,
            Node::InlineCode(inline_code) => write_text_element(writer, &inline_code.value)?,
            Node::Link(link) => write!(writer, "{}", strip_utm_source(&link.url))?,

            Node::Paragraph(paragraph) => {
                walk_mastodon(writer, math_formulae, paragraph.children)?;
                writeln!(writer)?;
            }
            Node::Heading(heading) => {
                walk_mastodon(writer, math_formulae, heading.children)?;
                writeln!(writer)?;
            }
            Node::List(list) => {
                writeln!(writer)?;
                walk_mastodon(writer, math_formulae, list.children)?;
                writeln!(writer)?;
            }
            Node::ListItem(list_item) => {
                write!(writer, "・")?;
                walk_mastodon(writer, math_formulae, list_item.children)?;
            }
            Node::Blockquote(blockquote) => {
                let mut quoted = String::new();
                walk_mastodon(&mut quoted, math_formulae, blockquote.children)?;
                for line in quoted.lines() {
                    writeln!(writer, "> ")?;
                    write_text_element(writer, line)?;
                    writeln!(writer)?;
                }
            }
            Node::Code(code) => {
                write_text_element(writer, &code.value)?;
                writeln!(writer)?;
            }

            Node::InlineMath(inline_math) => {
                let formula = inline_math.value.trim_matches('$').trim();
                math_formulae.push(formula.to_string());
                let reference = format!("(f.{})", math_formulae.len());
                write_text_element(writer, &reference)?;
            }
            Node::Math(math) => {
                let formula = math.value.trim_matches('$').trim();
                math_formulae.push(formula.to_string());
                let reference = format!("(f.{})\n", math_formulae.len());
                write_text_element(writer, &reference)?;
            }

            Node::Table(_) => {
                writeln!(writer, "(table omitted)")?;
            }

            _ => (),
        }
    }
    Ok(())
}

fn write_text_element(writer: &mut impl Write, original: &str) -> FmtResult {
    let escaped = RE_ESCAPING_MENTION.replace_all(original, "$1(at)$2");
    write!(writer, "{escaped}")?;
    Ok(())
}

fn strip_utm_source(url: &str) -> String {
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

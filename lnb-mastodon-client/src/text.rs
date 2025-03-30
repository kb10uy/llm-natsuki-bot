use std::{
    fmt::{Result as FmtResult, Write},
    sync::LazyLock,
};

use html2md::parse_html;
use markdown::{ParseOptions, mdast::Node};
use regex::Regex;
use url::Url;

static RE_HEAD_MENTION: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"^\s*\[@.+?\]\(.+?\)\s*"#).expect("invalid regex"));

pub fn sanitize_mention_html_from_mastodon(mention_html: &str) -> String {
    let content_markdown = parse_html(mention_html);
    RE_HEAD_MENTION.replace_all(&content_markdown, "").to_string()
}

pub fn sanitize_markdown_for_mastodon(markdown_text: &str) -> String {
    let markdown_ast = markdown::to_mdast(markdown_text, &ParseOptions { ..Default::default() })
        .expect("normal markdown parse never fails");
    let Node::Root(root) = markdown_ast else {
        unreachable!("root must be Node::Root");
    };

    let mut sanitized = String::new();
    walk_mastodon(&mut sanitized, root.children).expect("must succeed");
    sanitized
}

fn walk_mastodon(writer: &mut impl Write, children: Vec<Node>) -> FmtResult {
    for child in children {
        match child {
            Node::Root(root) => walk_mastodon(writer, root.children)?,

            Node::Text(text) => write!(writer, "{}", text.value)?,
            Node::Break(_) => writeln!(writer)?,
            Node::Strong(strong) => walk_mastodon(writer, strong.children)?,
            Node::Emphasis(emphasis) => walk_mastodon(writer, emphasis.children)?,
            Node::Delete(delete) => walk_mastodon(writer, delete.children)?,
            Node::InlineCode(inline_code) => write!(writer, "{}", inline_code.value)?,
            Node::InlineMath(inline_math) => write!(writer, "{}", inline_math.value)?,
            Node::Link(link) => write!(writer, "{}", strip_utm_source(&link.url))?,

            Node::Paragraph(paragraph) => {
                walk_mastodon(writer, paragraph.children)?;
                writeln!(writer)?;
            }
            Node::Heading(heading) => {
                walk_mastodon(writer, heading.children)?;
                writeln!(writer)?;
            }
            Node::List(list) => {
                writeln!(writer)?;
                walk_mastodon(writer, list.children)?;
                writeln!(writer)?;
            }
            Node::ListItem(list_item) => {
                write!(writer, "・")?;
                walk_mastodon(writer, list_item.children)?;
            }
            Node::Blockquote(blockquote) => {
                let mut quoted = String::new();
                walk_mastodon(&mut quoted, blockquote.children)?;
                for line in quoted.lines() {
                    writeln!(writer, "> {line}")?;
                }
            }
            Node::Code(code) => write!(writer, "{}", code.value)?,
            Node::Math(math) => write!(writer, "{}", math.value)?,

            Node::Table(_) => {
                writeln!(writer, "(table omitted)")?;
            }

            _ => (),
        }
    }
    Ok(())
}

fn strip_utm_source(url: &str) -> String {
    let Ok(parsed_url) = Url::parse(url) else {
        return url.to_string();
    };

    let stripped_url = if parsed_url.query().is_some() {
        let mut stripped = parsed_url.clone();
        let mut stripped_query = stripped.query_pairs_mut();
        stripped_query.clear();
        for (key, value) in parsed_url.query_pairs() {
            if key == "utm_source" {
                continue;
            }
            stripped_query.append_pair(&key, &value);
        }
        drop(stripped_query);
        stripped
    } else {
        parsed_url
    };

    stripped_url.to_string()
}

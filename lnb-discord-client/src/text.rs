use std::{
    fmt::{Result as FmtResult, Write},
    sync::LazyLock,
};

use markdown::{ParseOptions, mdast::Node};
use regex::Regex;

static RE_HEAD_MENTION: LazyLock<Regex> = LazyLock::new(|| Regex::new(r#"^\s*<@\d+?>\s*"#).expect("invalid regex"));

pub fn sanitize_discord_message(message: &str) -> String {
    RE_HEAD_MENTION.replace(message, "").to_string()
}

pub fn sanitize_markdown_for_discord(markdown_text: &str) -> String {
    let markdown_ast = markdown::to_mdast(markdown_text, &ParseOptions { ..Default::default() })
        .expect("normal markdown parse never fails");
    let Node::Root(root) = markdown_ast else {
        unreachable!("root must be Node::Root");
    };

    let mut sanitized = String::new();
    walk_discord(&mut sanitized, root.children).expect("must succeed");
    sanitized
}

fn walk_discord(writer: &mut impl Write, children: Vec<Node>) -> FmtResult {
    for child in children {
        match child {
            Node::Root(root) => walk_discord(writer, root.children)?,

            Node::Text(text) => write!(writer, "{}", text.value)?,
            Node::Break(_) => writeln!(writer)?,
            Node::Strong(strong) => walk_discord(writer, strong.children)?,
            Node::Emphasis(emphasis) => walk_discord(writer, emphasis.children)?,
            Node::Delete(delete) => walk_discord(writer, delete.children)?,
            Node::InlineCode(inline_code) => write!(writer, "{}", inline_code.value)?,
            Node::InlineMath(inline_math) => write!(writer, "{}", inline_math.value)?,
            Node::Link(link) => {
                write!(writer, "[")?;
                walk_discord(writer, link.children)?;
                write!(writer, "]({})", lnb_core::link::strip_utm_source(&link.url))?;
            }

            Node::Paragraph(paragraph) => {
                walk_discord(writer, paragraph.children)?;
                writeln!(writer)?;
            }
            Node::Heading(heading) => {
                walk_discord(writer, heading.children)?;
                writeln!(writer)?;
            }
            Node::List(list) => {
                writeln!(writer)?;
                walk_discord(writer, list.children)?;
                writeln!(writer)?;
            }
            Node::ListItem(list_item) => {
                write!(writer, "・")?;
                walk_discord(writer, list_item.children)?;
            }
            Node::Blockquote(blockquote) => {
                let mut quoted = String::new();
                walk_discord(&mut quoted, blockquote.children)?;
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

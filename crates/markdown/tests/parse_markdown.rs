//! Parsing GitHub-flavored Markdown into blocks, through the public API.

use qc_markdown::{
    ColumnAlignment, HeadingLevel, InlineStyle, MarkdownBlock, MarkdownDocument, StyledRun,
    TaskState,
};

fn blocks(source: &str) -> Vec<MarkdownBlock> {
    MarkdownDocument::parse(source).blocks().to_vec()
}

fn paragraph_text(block: &MarkdownBlock) -> &str {
    match block {
        MarkdownBlock::Paragraph(text) => text.text(),
        other => panic!("expected a paragraph, got {other:?}"),
    }
}

/// Headings keep their level and text; paragraphs join soft-wrapped lines with a space.
#[test]
fn parses_headings_and_paragraphs() {
    let parsed = blocks("# Title\n\nFirst line\nsecond line\n\n### Small");
    assert!(matches!(
        &parsed[0],
        MarkdownBlock::Heading { level: HeadingLevel::One, text } if text.text() == "Title"
    ));
    assert_eq!(paragraph_text(&parsed[1]), "First line second line");
    assert!(matches!(
        &parsed[2],
        MarkdownBlock::Heading {
            level: HeadingLevel::Three,
            ..
        }
    ));
}

/// Nested emphasis combines styles, and code spans are marked as code.
#[test]
fn combines_inline_styles() {
    let parsed = blocks("a **bold _both_** `code` ~~gone~~");
    let MarkdownBlock::Paragraph(text) = &parsed[0] else {
        panic!("expected a paragraph");
    };
    assert_eq!(text.text(), "a bold both code gone");
    assert_eq!(
        text.runs(),
        [
            StyledRun {
                range: 2..7,
                style: InlineStyle::STRONG
            },
            StyledRun {
                range: 7..11,
                style: InlineStyle::STRONG | InlineStyle::EMPHASIS
            },
            StyledRun {
                range: 12..16,
                style: InlineStyle::CODE
            },
            StyledRun {
                range: 17..21,
                style: InlineStyle::STRIKETHROUGH
            },
        ]
    );
}

/// Links record their URL over the link text's range.
#[test]
fn records_links() {
    let parsed = blocks("see [the docs](https://example.com) now");
    let MarkdownBlock::Paragraph(text) = &parsed[0] else {
        panic!("expected a paragraph");
    };
    assert_eq!(text.text(), "see the docs now");
    assert_eq!(text.links()[0].range, 4..12);
    assert_eq!(text.links()[0].url, "https://example.com");
    assert!(text.runs()[0].style.contains(InlineStyle::LINK));
}

/// An image becomes a link labeled with its alt text, or a placeholder without one.
#[test]
fn images_become_links() {
    let parsed = blocks("![screenshot](https://x/a.png) ![](https://x/b.png)");
    let MarkdownBlock::Paragraph(text) = &parsed[0] else {
        panic!("expected a paragraph");
    };
    assert_eq!(text.text(), "screenshot image");
    assert_eq!(text.links().len(), 2);
    assert_eq!(text.links()[1].url, "https://x/b.png");
}

/// Fenced code keeps its language and body without the closing newline.
#[test]
fn parses_fenced_code() {
    let parsed = blocks("```rust title=x\nfn main() {}\n```");
    assert_eq!(
        parsed[0],
        MarkdownBlock::CodeBlock {
            language: Some("rust".into()),
            code: "fn main() {}".into()
        }
    );
}

/// Tight list items get their text as a paragraph, and nested lists stay inside their item.
#[test]
fn parses_tight_and_nested_lists() {
    let parsed = blocks("1. one\n2. two\n   - inner\n");
    let MarkdownBlock::List(list) = &parsed[0] else {
        panic!("expected a list");
    };
    assert_eq!(list.start, Some(1));
    assert_eq!(list.items.len(), 2);
    assert_eq!(paragraph_text(&list.items[0].blocks[0]), "one");
    assert!(
        matches!(&list.items[1].blocks[1], MarkdownBlock::List(inner) if inner.start.is_none())
    );
}

/// Task list items carry their checkbox state.
#[test]
fn parses_task_lists() {
    let parsed = blocks("- [x] done\n- [ ] open\n- plain");
    let MarkdownBlock::List(list) = &parsed[0] else {
        panic!("expected a list");
    };
    let tasks: Vec<_> = list.items.iter().map(|item| item.task).collect();
    assert_eq!(tasks, [Some(TaskState::Done), Some(TaskState::Open), None]);
    assert_eq!(paragraph_text(&list.items[0].blocks[0]), "done");
}

/// Tables keep header, rows and alignments, with styled cell text.
#[test]
fn parses_tables() {
    let parsed = blocks("| Name | Count |\n|:-----|------:|\n| **a** | 1 |\n| b | 2 |");
    let MarkdownBlock::Table(table) = &parsed[0] else {
        panic!("expected a table");
    };
    assert_eq!(
        table.alignments,
        [ColumnAlignment::Left, ColumnAlignment::Right]
    );
    assert_eq!(table.header[1].text(), "Count");
    assert_eq!(table.rows.len(), 2);
    assert_eq!(table.rows[0][0].runs()[0].style, InlineStyle::STRONG);
}

/// Quotes hold their own blocks, and a rule is its own block.
#[test]
fn parses_quotes_and_rules() {
    let parsed = blocks("> quoted\n>\n> more\n\n---\n");
    let MarkdownBlock::BlockQuote(inner) = &parsed[0] else {
        panic!("expected a quote");
    };
    assert_eq!(inner.len(), 2);
    assert_eq!(parsed[1], MarkdownBlock::Rule);
}

/// A PR template's HTML comments vanish and `<details>` keeps its text.
#[test]
fn strips_html_from_templates() {
    let parsed = blocks(
        "<!-- Describe your change -->\n\nReal text\n\n<details>\n<summary>Logs</summary>\n\nline\n</details>\n",
    );
    let texts: Vec<_> = parsed.iter().map(paragraph_text).collect();
    assert_eq!(texts, ["Real text", "Logs", "line"]);
}

/// `<br>` inside a paragraph breaks the line.
#[test]
fn inline_br_breaks_line() {
    let parsed = blocks("one<br>two");
    assert_eq!(paragraph_text(&parsed[0]), "one\ntwo");
}

/// Empty input and whitespace give an empty document.
#[test]
fn empty_source_has_no_blocks() {
    assert!(MarkdownDocument::parse("  \n\n").is_empty());
}

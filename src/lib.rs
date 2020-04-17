use mdbook::book::{Book, BookItem, Chapter};
use mdbook::errors::{Error, Result};
use mdbook::preprocess::{Preprocessor, PreprocessorContext};
use pulldown_cmark::{Event, Options, Parser, Tag};
use pulldown_cmark_to_cmark::fmt::cmark;

pub struct Mermaid;

impl Preprocessor for Mermaid {
    fn name(&self) -> &str {
        "mermaid"
    }

    fn run(&self, _ctx: &PreprocessorContext, mut book: Book) -> Result<Book> {
        apply_to_sections(&mut book.sections)?;

        Ok(book)
    }

    fn supports_renderer(&self, renderer: &str) -> bool {
        renderer == "html"
    }
}

fn apply_to_sections(sections: &mut [BookItem]) -> Result<()> {
    for section in sections {
        match section {
            BookItem::Chapter(ch) => apply_to_chapter(ch)?,
            BookItem::Separator => {}
        }
    }

    Ok(())
}

fn apply_to_chapter(chapter: &mut Chapter) -> Result<()> {
    chapter.content = add_mermaid(&chapter.content)?;

    apply_to_sections(&mut chapter.sub_items)
}

fn add_mermaid(content: &str) -> Result<String> {
    let mut buf = String::with_capacity(content.len());
    let mut mermaid_content = String::new();
    let mut in_mermaid_block = false;

    let mut opts = Options::empty();
    opts.insert(Options::ENABLE_TABLES);
    opts.insert(Options::ENABLE_FOOTNOTES);
    opts.insert(Options::ENABLE_STRIKETHROUGH);
    opts.insert(Options::ENABLE_TASKLISTS);

    let events = Parser::new_ext(content, opts).map(|e| {
        if let Event::Start(Tag::CodeBlock(code)) = e.clone() {
            if &*code == "mermaid" {
                in_mermaid_block = true;
                mermaid_content.clear();
                return None;
            } else {
                return Some(e);
            }
        }

        if !in_mermaid_block {
            return Some(e);
        }

        match e {
            Event::End(Tag::CodeBlock(code)) => {
                assert_eq!(
                    "mermaid", &*code,
                    "After an opening mermaid code block we expect it to close again"
                );
                in_mermaid_block = false;

                let mermaid_code = format!("<pre class=\"mermaid\">{}</pre>\n\n", mermaid_content);
                return Some(Event::Html(mermaid_code.into()));
            }
            Event::Text(code) => {
                mermaid_content.push_str(&code);
            }
            _ => return Some(e),
        }

        None
    });
    let events = events.filter_map(|e| e);
    cmark(events, &mut buf, None)
        .map(|_| buf)
        .map_err(|err| Error::from(format!("Markdown serialization failed: {}", err)))
}

#[cfg(test)]
mod test {
    use super::add_mermaid;

    #[test]
    fn adds_mermaid() {
        let content = r#"# Chapter

```mermaid
graph TD
A --> B
```

Text
"#;

        let expected = r#"# Chapter

<pre class="mermaid">graph TD
A --> B
</pre>

Text"#;

        assert_eq!(expected, add_mermaid(content).unwrap());
    }

    #[test]
    fn leaves_tables_untouched() {
        // Regression test.
        // Previously we forgot to enable the same markdwon extensions as mdbook itself.

        let content = r#"# Heading

| Head 1 | Head 2 |
|--------|--------|
| Row 1  | Row 2  |
"#;

        // Markdown roundtripping removes some insignificant whitespace
        let expected = r#"# Heading

|Head 1|Head 2|
|------|------|
|Row 1|Row 2|"#;

        assert_eq!(expected, add_mermaid(content).unwrap());
    }
}

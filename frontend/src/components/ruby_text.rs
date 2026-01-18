use yew::prelude::*;

pub enum TextSegment {
    Plain(String),
    Ruby { base: String, reading: String },
}

pub fn parse_ruby_text(text: &str) -> Vec<TextSegment> {
    let mut segments = Vec::new();
    let mut current_pos = 0;

    while let Some(ruby_start) = text[current_pos..].find("<ruby>") {
        let abs_start = current_pos + ruby_start;
        if abs_start > current_pos {
            segments.push(TextSegment::Plain(text[current_pos..abs_start].to_string()));
        }

        if let Some(rt_start) = text[abs_start..].find("<rt>") {
            let abs_rt_start = abs_start + rt_start;
            let base = text[abs_start + 6..abs_rt_start].to_string();

            if let Some(rt_end) = text[abs_rt_start..].find("</rt>") {
                let abs_rt_end = abs_rt_start + rt_end;
                let reading = text[abs_rt_start + 4..abs_rt_end].to_string();

                if let Some(ruby_end) = text[abs_rt_end..].find("</ruby>") {
                    let abs_ruby_end = abs_rt_end + ruby_end;
                    segments.push(TextSegment::Ruby { base, reading });
                    current_pos = abs_ruby_end + 7; // Skip past </ruby>
                } else {
                    break;
                }
            } else {
                break;
            }
        } else {
            break;
        }
    }

    if current_pos < text.len() {
        segments.push(TextSegment::Plain(text[current_pos..].to_string()));
    }

    segments
}

pub fn render_text_with_ruby(text: &str) -> Html {
    let segments = parse_ruby_text(text);
    html! {
        <>
            { for segments.iter().map(|seg| match seg {
                TextSegment::Plain(s) => html! { <>{s}</> },
                TextSegment::Ruby { base, reading } => html! {
                    <ruby>{base}<rt>{reading}</rt></ruby>
                },
            })}
        </>
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_ruby_text_plain_only() {
        let segments = parse_ruby_text("Hello world");
        assert_eq!(segments.len(), 1);
        match &segments[0] {
            TextSegment::Plain(s) => assert_eq!(s, "Hello world"),
            _ => panic!("Expected plain text"),
        }
    }

    #[test]
    fn test_parse_ruby_text_single_ruby() {
        let segments = parse_ruby_text("<ruby>漢字<rt>かんじ</rt></ruby>");
        assert_eq!(segments.len(), 1);
        match &segments[0] {
            TextSegment::Ruby { base, reading } => {
                assert_eq!(base, "漢字");
                assert_eq!(reading, "かんじ");
            }
            _ => panic!("Expected ruby segment"),
        }
    }

    #[test]
    fn test_parse_ruby_text_mixed() {
        let text = "Hello <ruby>世界<rt>せかい</rt></ruby>!";
        let segments = parse_ruby_text(text);
        assert_eq!(segments.len(), 3);

        match &segments[0] {
            TextSegment::Plain(s) => assert_eq!(s, "Hello "),
            _ => panic!("Expected plain text"),
        }

        match &segments[1] {
            TextSegment::Ruby { base, reading } => {
                assert_eq!(base, "世界");
                assert_eq!(reading, "せかい");
            }
            _ => panic!("Expected ruby segment"),
        }

        match &segments[2] {
            TextSegment::Plain(s) => assert_eq!(s, "!"),
            _ => panic!("Expected plain text"),
        }
    }

    #[test]
    fn test_parse_ruby_text_multiple_ruby() {
        let text = "<ruby>今日<rt>きょう</rt></ruby>は<ruby>何<rt>なに</rt></ruby>";
        let segments = parse_ruby_text(text);
        assert_eq!(segments.len(), 3);

        match &segments[0] {
            TextSegment::Ruby { base, reading } => {
                assert_eq!(base, "今日");
                assert_eq!(reading, "きょう");
            }
            _ => panic!("Expected ruby segment"),
        }

        match &segments[1] {
            TextSegment::Plain(s) => assert_eq!(s, "は"),
            _ => panic!("Expected plain text"),
        }

        match &segments[2] {
            TextSegment::Ruby { base, reading } => {
                assert_eq!(base, "何");
                assert_eq!(reading, "なに");
            }
            _ => panic!("Expected ruby segment"),
        }
    }
}

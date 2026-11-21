//! Find and replace over type-layer text. The matcher works on the original
//! text, so a replacement is never rescanned; `forward` picks the first
//! non-overlapping match left-to-right, or the last right-to-left.

/// The dialog's matching options.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReplaceOptions {
    pub forward: bool,
    pub case_sensitive: bool,
    pub whole_word: bool,
}

/// Replace every match of `find` with `replacement`, returning the new text and
/// the number of matches. An empty `find` changes nothing.
pub fn replace_matches(
    text: &str,
    find: &str,
    replacement: &str,
    options: &ReplaceOptions,
) -> (String, usize) {
    if find.is_empty() {
        return (text.to_string(), 0);
    }
    let mut matches = Vec::new();
    if options.forward {
        let mut cursor = 0;
        while let Some((start, end)) = forward_match(text, cursor, find, options) {
            matches.push((start, end));
            cursor = end;
        }
    } else {
        let mut cursor = text.len();
        while let Some((start, end)) = backward_match(text, cursor, find, options) {
            matches.push((start, end));
            cursor = start;
        }
        matches.reverse();
    }
    let mut out = String::with_capacity(text.len());
    let mut cursor = 0;
    for (start, end) in &matches {
        out.push_str(&text[cursor..*start]);
        out.push_str(replacement);
        cursor = *end;
    }
    out.push_str(&text[cursor..]);
    (out, matches.len())
}

fn forward_match(
    text: &str,
    from: usize,
    find: &str,
    options: &ReplaceOptions,
) -> Option<(usize, usize)> {
    for (offset, _) in text[from..].char_indices() {
        let start = from + offset;
        if let Some(end) = match_at(text, start, find, options) {
            return Some((start, end));
        }
    }
    None
}

fn backward_match(
    text: &str,
    before: usize,
    find: &str,
    options: &ReplaceOptions,
) -> Option<(usize, usize)> {
    for (start, _) in text[..before].char_indices().rev() {
        if let Some(end) = match_at(text, start, find, options) {
            if end <= before {
                return Some((start, end));
            }
        }
    }
    None
}

fn match_at(text: &str, start: usize, find: &str, options: &ReplaceOptions) -> Option<usize> {
    let mut end = start;
    let mut text_chars = text[start..].chars();
    for needle in find.chars() {
        let candidate = text_chars.next()?;
        let equal = if options.case_sensitive {
            candidate == needle
        } else {
            candidate.eq_ignore_ascii_case(&needle)
        };
        if !equal {
            return None;
        }
        end += candidate.len_utf8();
    }
    if options.whole_word {
        if text[..start].chars().next_back().is_some_and(is_word_char) {
            return None;
        }
        if text[end..].chars().next().is_some_and(is_word_char) {
            return None;
        }
    }
    Some(end)
}

fn is_word_char(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

#[cfg(test)]
mod tests {
    use super::*;

    fn options(forward: bool, case_sensitive: bool, whole_word: bool) -> ReplaceOptions {
        ReplaceOptions {
            forward,
            case_sensitive,
            whole_word,
        }
    }

    #[test]
    fn replace_matches_case() {
        let text = "cat Cat Cat";
        let (out, count) = replace_matches(text, "cat", "dog", &options(true, true, false));
        assert_eq!((out.as_str(), count), ("dog Cat Cat", 1));
        let (out, count) = replace_matches(text, "cat", "dog", &options(true, false, false));
        assert_eq!((out.as_str(), count), ("dog dog dog", 3));
    }

    #[test]
    fn replace_matches_whole_word() {
        let text = "cat catalog cat_ catalog";
        let (out, count) = replace_matches(text, "cat", "dog", &options(true, true, true));
        assert_eq!((out.as_str(), count), ("dog catalog cat_ catalog", 1));
    }

    #[test]
    fn replace_matches_forward() {
        let (out, count) = replace_matches("aaa", "aa", "X", &options(true, true, false));
        assert_eq!((out.as_str(), count), ("Xa", 1));
        let (out, count) = replace_matches("aaa", "aa", "X", &options(false, true, false));
        assert_eq!((out.as_str(), count), ("aX", 1));
    }

    #[test]
    fn replace_matches_no_match() {
        let (out, count) = replace_matches("abc", "z", "x", &options(true, true, false));
        assert_eq!((out.as_str(), count), ("abc", 0));
        let (out, count) = replace_matches("abc", "", "x", &options(true, true, false));
        assert_eq!((out.as_str(), count), ("abc", 0));
    }

    #[test]
    fn replace_matches_empty_find() {
        let (out, count) = replace_matches("abc", "", "x", &options(true, true, false));
        assert_eq!((out.as_str(), count), ("abc", 0));
    }
}

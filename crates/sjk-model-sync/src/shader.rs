//! The names a `.shader` file defines: each word outside braces, read the way the
//! game's parser reads them (`//` and `/* */` comments, `{` `}` as their own tokens).

/// Every shader name `text` defines, or why its braces do not add up.
pub(crate) fn names(text: &str) -> Result<Vec<String>, String> {
    let mut names = Vec::new();
    let mut depth = 0usize;
    for token in tokens(text)? {
        match token {
            "{" => depth += 1,
            "}" => {
                depth = depth
                    .checked_sub(1)
                    .ok_or_else(|| "a } closes nothing".to_owned())?;
            }
            name if depth == 0 => names.push(name.to_owned()),
            _ => {}
        }
    }
    if depth != 0 {
        return Err("a { is never closed".into());
    }
    Ok(names)
}

fn tokens(text: &str) -> Result<Vec<&str>, String> {
    let mut tokens = Vec::new();
    let bytes = text.as_bytes();
    let mut at = 0;
    while at < bytes.len() {
        let rest = &text[at..];
        if bytes[at].is_ascii_whitespace() {
            at += 1;
        } else if rest.starts_with("//") {
            at += rest.find('\n').unwrap_or(rest.len());
        } else if rest.starts_with("/*") {
            let end = rest.find("*/").ok_or("a /* comment is never closed")?;
            at += end + 2;
        } else if matches!(bytes[at], b'{' | b'}') {
            tokens.push(&rest[..1]);
            at += 1;
        } else {
            let end = rest
                .find(|c: char| c.is_ascii_whitespace() || c == '{' || c == '}')
                .unwrap_or(rest.len());
            // A comment glued to a word ends it, as in the game's parser.
            let end = [rest[..end].find("//"), rest[..end].find("/*")]
                .into_iter()
                .flatten()
                .min()
                .unwrap_or(end)
                .max(1);
            tokens.push(&rest[..end]);
            at += end;
        }
    }
    Ok(tokens)
}

/// `text` with only the shaders whose names start with `prefix` (lower case), each
/// with its comments before it; or why its braces do not add up.
pub(crate) fn keep_own(text: &str, prefix: &str) -> Result<String, String> {
    let mut kept = String::new();
    let mut depth = 0usize;
    // The start of the current top-level shader's text (after the last one's end).
    let mut start = 0usize;
    let mut own = false;
    for (at, token) in spans(text)? {
        match token {
            "{" => depth += 1,
            "}" => {
                depth = depth
                    .checked_sub(1)
                    .ok_or_else(|| "a } closes nothing".to_owned())?;
                if depth == 0 {
                    let end = at + 1;
                    if own {
                        kept.push_str(text[start..end].trim_start_matches(['\r', '\n']));
                        kept.push('\n');
                    }
                    start = end;
                }
            }
            name if depth == 0 => own = name.to_ascii_lowercase().starts_with(prefix),
            _ => {}
        }
    }
    if depth != 0 {
        return Err("a { is never closed".into());
    }
    Ok(kept)
}

/// [`tokens`] with each token's byte offset.
fn spans(text: &str) -> Result<Vec<(usize, &str)>, String> {
    let base = text.as_ptr() as usize;
    Ok(tokens(text)?
        .into_iter()
        .map(|token| (token.as_ptr() as usize - base, token))
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_the_models_own_shaders_are_kept() {
        let text = "// Krovos\nmodels/players/krovos/body\n{\n\tmap a // }\n}\n\n\
                    models/players/krovos2/lashes\n{\n\t{ map b }\n}\n\
                    models/players/krovos/eyes { map c }\n";
        let kept = keep_own(text, "models/players/krovos/").unwrap();
        assert_eq!(
            names(&kept).unwrap(),
            ["models/players/krovos/body", "models/players/krovos/eyes"]
        );
        assert!(kept.contains("// Krovos") && !kept.contains("lashes"));
        assert!(keep_own("a {", "a").is_err());
    }
}

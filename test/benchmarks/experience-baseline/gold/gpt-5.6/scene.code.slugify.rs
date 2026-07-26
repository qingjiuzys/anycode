pub fn slugify(input: &str) -> String {
    let mut output = String::with_capacity(input.len());
    let mut pending_separator = false;

    for ch in input.chars().flat_map(char::to_lowercase) {
        if ch.is_ascii_lowercase() || ch.is_ascii_digit() {
            if pending_separator && !output.is_empty() {
                output.push('-');
            }
            output.push(ch);
            pending_separator = false;
        } else if ch.is_whitespace() || ch == '-' {
            pending_separator = !output.is_empty();
        }
    }

    output
}

#[cfg(test)]
mod tests {
    use super::slugify;

    #[test]
    fn slugifies_basic_phrase() {
        assert_eq!(slugify("Hello World"), "hello-world");
    }

    #[test]
    fn strips_punctuation() {
        assert_eq!(slugify("Rust, 2026: Fast!"), "rust-2026-fast");
    }

    #[test]
    fn collapses_multiple_spaces() {
        assert_eq!(slugify("one   two\t\nthree"), "one-two-three");
    }

    #[test]
    fn trims_leading_and_trailing_junk() {
        assert_eq!(slugify("---*** Hello World !!!---"), "hello-world");
    }
}

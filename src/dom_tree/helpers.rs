pub(crate) fn normalized_char_count(text: &str, start_whitespace: bool) -> usize {
    let mut char_count = 0;
    let mut prev_was_whitespace = start_whitespace;

    for c in text.chars() {
        if !(prev_was_whitespace && c.is_whitespace()) {
            char_count += 1;
        }
        prev_was_whitespace = c.is_whitespace();
    }
    char_count
}

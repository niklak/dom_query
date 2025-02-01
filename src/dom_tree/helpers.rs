pub(crate) fn normalized_char_count(text: &str) -> usize {
    let mut char_count = 0;
    let mut prev_was_whitespace = true;

    for c in text.chars() {
        if prev_was_whitespace && c.is_whitespace() {
            continue;
        }
        char_count += 1;
        prev_was_whitespace = c.is_whitespace();
    }

    if prev_was_whitespace && char_count > 0 {
        char_count -= 1;
    }

    char_count
}

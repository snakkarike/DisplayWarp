pub fn truncate_text(text: &str, max_chars: usize) -> String {
    if text.chars().count() > max_chars {
        text.chars().take(max_chars - 1).collect::<String>() + "…"
    } else {
        text.to_string()
    }
}

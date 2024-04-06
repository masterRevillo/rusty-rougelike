
pub fn split_str(s: &str, line_length: usize) -> Vec<String> {
    let mut result = Vec::new();
    let mut buf = String::new();
    s.split_whitespace().for_each(|word| {
        buf.push_str(" ");
        buf.push_str(word);
        if buf.len() > line_length {
            result.push(buf.to_string());
            buf = String::new();
        }
    });
    if !buf.is_empty() {
        result.push(buf);
    }
    result
}

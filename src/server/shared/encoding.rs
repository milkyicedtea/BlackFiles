pub(crate) fn url_decode(value: &str) -> String {
    let mut decoded = String::with_capacity(value.len());
    let mut chars = value.chars();

    while let Some(character) = chars.next() {
        match character {
            '%' => {
                let hex: String = chars.by_ref().take(2).collect();
                if let Ok(byte) = u8::from_str_radix(&hex, 16) {
                    decoded.push(byte as char);
                } else {
                    decoded.push('%');
                    decoded.push_str(&hex);
                }
            }
            '+' => decoded.push(' '),
            _ => decoded.push(character),
        }
    }

    decoded
}

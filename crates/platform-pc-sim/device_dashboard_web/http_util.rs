use std::io::Write as _;
use std::net::TcpStream;

/// Send an HTTP response with the given status, content-type, and body.
pub(super) fn respond(stream: &mut TcpStream, status: &str, content_type: &str, body: &str) {
    let header = format!(
        "HTTP/1.1 {status}\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        body.len()
    );
    // Write header and body separately so the (potentially large) body is
    // never copied into a fresh allocation just to prepend a header line.
    if stream.write_all(header.as_bytes()).is_err() {
        return;
    }
    let _ = stream.write_all(body.as_bytes());
}

/// Extract a string field value from a minimal JSON body.
///
/// Handles `{"key":"value"}` without a full JSON parser.
pub(super) fn parse_json_string_field<'a>(json: &'a str, key: &str) -> Option<&'a str> {
    let key_literal = format!("\"{key}\"");
    let after_key = json.split(key_literal.as_str()).nth(1)?;
    let after_colon = after_key.split(':').nth(1)?.trim_start();
    let inner = after_colon.strip_prefix('"')?;
    let end = inner.find('"')?;
    Some(&inner[..end])
}

/// Extract the `"board"` string value from a minimal JSON object.
///
/// Handles `{"board":"arduino-nano"}` without pulling in a full JSON parser.
pub(super) fn parse_board_from_json(json: &str) -> Option<&str> {
    parse_json_string_field(json, "board")
}

/// Extract `sensor_profile` field from a JSON body string.
///
/// Handles `{"sensor_profile":"climate"}` without a full JSON parser.
pub(super) fn parse_sensor_profile_from_json(json: &str) -> Option<&str> {
    parse_json_string_field(json, "sensor_profile")
}

pub(super) fn parse_json_string_array_field(json: &str, key: &str) -> Option<Vec<String>> {
    let key_literal = format!("\"{key}\"");
    let after_key = json.split(key_literal.as_str()).nth(1)?;
    let after_colon = after_key.split(':').nth(1)?.trim_start();
    let inner = after_colon.strip_prefix('[')?;
    let end = inner.find(']')?;
    let values = inner[..end].trim();
    if values.is_empty() {
        return Some(vec![]);
    }

    Some(
        values
            .split(',')
            .filter_map(|entry| {
                let trimmed = entry.trim();
                let without_prefix = trimmed.strip_prefix('"')?;
                let end = without_prefix.find('"')?;
                Some(without_prefix[..end].to_string())
            })
            .collect(),
    )
}

pub(super) fn parse_json_bool_field(json: &str, key: &str) -> Option<bool> {
    let key_literal = format!("\"{key}\"");
    let after_key = json.split(key_literal.as_str()).nth(1)?;
    let after_colon = after_key.split(':').nth(1)?.trim_start();
    if after_colon.starts_with("true") {
        Some(true)
    } else if after_colon.starts_with("false") {
        Some(false)
    } else {
        None
    }
}

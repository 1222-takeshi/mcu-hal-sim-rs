use std::fmt;
use std::io::{self, BufRead};

pub const MAX_OTA_SIZE: usize = 0x1E0000;
pub const MAX_HEADER_LINE_LEN: usize = 512;
pub const MAX_HEADER_BYTES: usize = 2048;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RequestLimits {
    pub max_payload_size: usize,
    pub max_header_line_len: usize,
    pub max_header_bytes: usize,
}

impl Default for RequestLimits {
    fn default() -> Self {
        Self {
            max_payload_size: MAX_OTA_SIZE,
            max_header_line_len: MAX_HEADER_LINE_LEN,
            max_header_bytes: MAX_HEADER_BYTES,
        }
    }
}

#[derive(Debug)]
pub enum OtaRequestError {
    BadRequest,
    Unauthorized,
    HeaderTooLarge,
    LengthRequired,
    PayloadTooLarge(usize),
    Io(io::Error),
}

impl From<io::Error> for OtaRequestError {
    fn from(value: io::Error) -> Self {
        Self::Io(value)
    }
}

impl fmt::Display for OtaRequestError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::BadRequest => write!(f, "bad request"),
            Self::Unauthorized => write!(f, "missing or invalid OTA token"),
            Self::HeaderTooLarge => write!(f, "request header too large"),
            Self::LengthRequired => write!(f, "missing or invalid Content-Length"),
            Self::PayloadTooLarge(len) => write!(f, "payload too large: {len} bytes"),
            Self::Io(err) => write!(f, "I/O error while reading request: {err}"),
        }
    }
}

pub fn read_ota_request<R: BufRead>(
    reader: &mut R,
    expected_token: &str,
    limits: RequestLimits,
) -> Result<usize, OtaRequestError> {
    let request_line = read_header_line(reader, limits.max_header_line_len)?;
    if !request_line.terminated {
        return Err(OtaRequestError::BadRequest);
    }
    let mut header_bytes = request_line.text.len();
    if header_bytes > limits.max_header_bytes {
        return Err(OtaRequestError::HeaderTooLarge);
    }

    let request_line = request_line.text.trim_end_matches(['\r', '\n']);
    let mut parts = request_line.split_ascii_whitespace();
    let valid_request_line = matches!(
        (parts.next(), parts.next(), parts.next(), parts.next()),
        (
            Some("POST"),
            Some("/ota"),
            Some("HTTP/1.0" | "HTTP/1.1"),
            None
        )
    );
    if !valid_request_line {
        return Err(OtaRequestError::BadRequest);
    }

    let mut content_length = None;
    let mut content_length_seen = false;
    let mut token_matches = false;
    let mut token_seen = false;

    loop {
        let line = read_header_line(reader, limits.max_header_line_len)?;
        if !line.terminated {
            return Err(OtaRequestError::BadRequest);
        }

        header_bytes = header_bytes
            .checked_add(line.text.len())
            .ok_or(OtaRequestError::HeaderTooLarge)?;
        if header_bytes > limits.max_header_bytes {
            return Err(OtaRequestError::HeaderTooLarge);
        }

        let trimmed = line.text.trim_end_matches(['\r', '\n']);
        if trimmed.is_empty() {
            break;
        }

        let (name, value) = trimmed.split_once(':').ok_or(OtaRequestError::BadRequest)?;
        if name.is_empty() {
            return Err(OtaRequestError::BadRequest);
        }

        if name.eq_ignore_ascii_case("content-length") {
            if content_length_seen {
                return Err(OtaRequestError::BadRequest);
            }
            content_length_seen = true;
            content_length = value.trim().parse::<usize>().ok();
        } else if name.eq_ignore_ascii_case("x-ota-token") {
            if token_seen {
                return Err(OtaRequestError::BadRequest);
            }
            token_seen = true;
            token_matches = value.trim() == expected_token;
        }
    }

    if !token_matches {
        return Err(OtaRequestError::Unauthorized);
    }

    let content_length = content_length.ok_or(OtaRequestError::LengthRequired)?;
    if content_length == 0 || content_length > limits.max_payload_size {
        return Err(OtaRequestError::PayloadTooLarge(content_length));
    }

    Ok(content_length)
}

struct HeaderLine {
    text: String,
    terminated: bool,
}

fn read_header_line<R: BufRead>(
    reader: &mut R,
    max_line_len: usize,
) -> Result<HeaderLine, OtaRequestError> {
    let mut line = Vec::new();
    let mut terminated = false;

    loop {
        let available = reader.fill_buf()?;
        if available.is_empty() {
            break;
        }

        let newline = available.iter().position(|byte| *byte == b'\n');
        let take_len = newline.map_or(available.len(), |position| position + 1);
        if line.len().saturating_add(take_len) > max_line_len {
            return Err(OtaRequestError::HeaderTooLarge);
        }

        line.extend_from_slice(&available[..take_len]);
        reader.consume(take_len);

        if newline.is_some() {
            terminated = true;
            break;
        }
    }

    let text = String::from_utf8(line).map_err(|_| OtaRequestError::BadRequest)?;
    Ok(HeaderLine { text, terminated })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{self, BufRead, Cursor, Read};

    const TOKEN: &str = "test-token";

    fn parse(request: &[u8]) -> Result<usize, OtaRequestError> {
        let mut reader = Cursor::new(request);
        read_ota_request(&mut reader, TOKEN, RequestLimits::default())
    }

    fn valid_request(version: &str, content_length: usize) -> Vec<u8> {
        format!(
            "POST /ota {version}\r\nContent-Length: {content_length}\r\nX-OTA-Token: {TOKEN}\r\n\r\n"
        )
        .into_bytes()
    }

    #[test]
    fn accepts_http_1_0_and_1_1() {
        assert_eq!(parse(&valid_request("HTTP/1.0", 42)).unwrap(), 42);
        assert_eq!(parse(&valid_request("HTTP/1.1", 43)).unwrap(), 43);
    }

    #[test]
    fn accepts_case_insensitive_header_names_and_trimmed_values() {
        let request =
            b"POST /ota HTTP/1.1\r\ncontent-length: 7 \r\nx-ota-token:  test-token \r\n\r\n";
        assert_eq!(parse(request).unwrap(), 7);
    }

    #[test]
    fn rejects_missing_or_invalid_token() {
        let missing = b"POST /ota HTTP/1.1\r\nContent-Length: 7\r\n\r\n";
        let invalid = b"POST /ota HTTP/1.1\r\nContent-Length: 7\r\nX-OTA-Token: wrong\r\n\r\n";

        assert!(matches!(parse(missing), Err(OtaRequestError::Unauthorized)));
        assert!(matches!(parse(invalid), Err(OtaRequestError::Unauthorized)));
    }

    #[test]
    fn rejects_invalid_request_line() {
        for line in [
            "GET /ota HTTP/1.1",
            "POST /wrong HTTP/1.1",
            "POST /ota HTTP/2.0",
            "POST /ota HTTP/1.1 extra",
        ] {
            let request = format!("{line}\r\nContent-Length: 7\r\nX-OTA-Token: {TOKEN}\r\n\r\n");
            assert!(matches!(
                parse(request.as_bytes()),
                Err(OtaRequestError::BadRequest)
            ));
        }
    }

    #[test]
    fn rejects_missing_or_invalid_content_length() {
        let missing = format!("POST /ota HTTP/1.1\r\nX-OTA-Token: {TOKEN}\r\n\r\n");
        let invalid =
            format!("POST /ota HTTP/1.1\r\nContent-Length: NaN\r\nX-OTA-Token: {TOKEN}\r\n\r\n");

        assert!(matches!(
            parse(missing.as_bytes()),
            Err(OtaRequestError::LengthRequired)
        ));
        assert!(matches!(
            parse(invalid.as_bytes()),
            Err(OtaRequestError::LengthRequired)
        ));
    }

    #[test]
    fn enforces_payload_boundaries() {
        assert!(matches!(
            parse(&valid_request("HTTP/1.1", 0)),
            Err(OtaRequestError::PayloadTooLarge(0))
        ));
        assert_eq!(
            parse(&valid_request("HTTP/1.1", MAX_OTA_SIZE)).unwrap(),
            MAX_OTA_SIZE
        );
        assert!(matches!(
            parse(&valid_request("HTTP/1.1", MAX_OTA_SIZE + 1)),
            Err(OtaRequestError::PayloadTooLarge(len)) if len == MAX_OTA_SIZE + 1
        ));
    }

    #[test]
    fn rejects_header_line_over_limit() {
        let long_value = "a".repeat(MAX_HEADER_LINE_LEN);
        let request = format!(
            "POST /ota HTTP/1.1\r\nX-Long: {long_value}\r\nContent-Length: 7\r\nX-OTA-Token: {TOKEN}\r\n\r\n"
        );

        assert!(matches!(
            parse(request.as_bytes()),
            Err(OtaRequestError::HeaderTooLarge)
        ));
    }

    #[test]
    fn rejects_total_headers_over_limit() {
        let mut request = String::from("POST /ota HTTP/1.1\r\n");
        while request.len() <= MAX_HEADER_BYTES {
            request.push_str("X-Padding: 012345678901234567890123456789\r\n");
        }

        assert!(matches!(
            parse(request.as_bytes()),
            Err(OtaRequestError::HeaderTooLarge)
        ));
    }

    #[test]
    fn rejects_invalid_utf8_and_eof_before_header_terminator() {
        let mut invalid_utf8 = valid_request("HTTP/1.1", 7);
        invalid_utf8.splice(22..22, [0xff]);
        let early_eof =
            format!("POST /ota HTTP/1.1\r\nContent-Length: 7\r\nX-OTA-Token: {TOKEN}\r\n");

        assert!(matches!(
            parse(&invalid_utf8),
            Err(OtaRequestError::BadRequest)
        ));
        assert!(matches!(
            parse(early_eof.as_bytes()),
            Err(OtaRequestError::BadRequest)
        ));
    }

    #[test]
    fn rejects_duplicate_or_malformed_security_headers() {
        let duplicate_length = format!(
            "POST /ota HTTP/1.1\r\nContent-Length: 7\r\nContent-Length: 7\r\nX-OTA-Token: {TOKEN}\r\n\r\n"
        );
        let duplicate_token = format!(
            "POST /ota HTTP/1.1\r\nContent-Length: 7\r\nX-OTA-Token: {TOKEN}\r\nX-OTA-Token: {TOKEN}\r\n\r\n"
        );
        let malformed =
            format!("POST /ota HTTP/1.1\r\nContent-Length: 7\r\nX-OTA-Token {TOKEN}\r\n\r\n");

        for request in [duplicate_length, duplicate_token, malformed] {
            assert!(matches!(
                parse(request.as_bytes()),
                Err(OtaRequestError::BadRequest)
            ));
        }
    }

    struct FailingReader;

    impl Read for FailingReader {
        fn read(&mut self, _buf: &mut [u8]) -> io::Result<usize> {
            Err(io::Error::other("test failure"))
        }
    }

    impl BufRead for FailingReader {
        fn fill_buf(&mut self) -> io::Result<&[u8]> {
            Err(io::Error::other("test failure"))
        }

        fn consume(&mut self, _amt: usize) {}
    }

    #[test]
    fn preserves_reader_errors() {
        let error = read_ota_request(&mut FailingReader, TOKEN, RequestLimits::default())
            .expect_err("reader failure must be returned");

        assert!(matches!(error, OtaRequestError::Io(_)));
    }
}

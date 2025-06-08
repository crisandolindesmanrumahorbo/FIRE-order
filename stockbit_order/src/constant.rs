pub const UNAUTHORIZED: &str = "HTTP/1.1 401 Unauthorized\r\n\r\n";
pub const NOT_FOUND: &str = "HTTP/1.1 404 NOT FOUND\r\n\r\n";
pub const BAD_REQUEST: &str = "HTTP/1.1 400 Bad Request\r\n\r\n";
pub const OK_RESPONSE: &str = "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\n\r\n";
pub const INTERNAL_ERROR: &str = "HTTP/1.1 500 Internal Error\r\n\r\n";

pub const LOGGING_INCOMING_REQUEST: &str = "Incoming Request handling by: ";
pub const LOGGING_HANDSHAKE: &str = "Handshake handling by: ";
pub const LOGGING_MESSAGE: &str = "Message handling by: ";

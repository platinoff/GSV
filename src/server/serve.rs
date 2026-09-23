//! HTTP/1 accept loop.
//! On this machine a stream filter rewrites `Content-Length` to
//! `Transfer-Encoding: chunked` for request-target `/mcp` and leaves the body
//! unframed (`{` where a chunk size should be). Cursor and curl then drop the
//! session. Finite responses are therefore real HTTP/1 chunks. Session GET SSE
//! is still a raw event-stream.

use std::io::ErrorKind;
use std::pin::Pin;

use axum::body::{to_bytes, Body, Bytes, HttpBody};
use axum::http::{header, HeaderName, HeaderValue, Method, Request, StatusCode};
use axum::response::Response;
use axum::Router;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tower::ServiceExt;

const MCP_FINITE_MAX: usize = 8 * 1024 * 1024;
const HEADER_MAX: usize = 64 * 1024;

/// Serve `app` on `listener`. Live `gsv-server` uses this instead of
/// `axum::serve` so Cursor Streamable HTTP can parse `/mcp`.
pub async fn serve(listener: tokio::net::TcpListener, app: Router) -> std::io::Result<()> {
    loop {
        let (io, _) = listener.accept().await?;
        let app = app.clone();
        tokio::spawn(async move {
            let _ = handle_conn(io, app).await;
        });
    }
}

async fn handle_conn(mut stream: TcpStream, app: Router) -> std::io::Result<()> {
    loop {
        let req = match read_request(&mut stream).await? {
            Some(req) => req,
            None => return Ok(()),
        };
        let res = match app.clone().oneshot(req).await {
            Ok(res) => res,
            Err(e) => match e {},
        };
        let sse = res
            .headers()
            .get(header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .is_some_and(|ct| ct.starts_with("text/event-stream"));
        let has_len = res.headers().contains_key(header::CONTENT_LENGTH);
        if sse && !has_len {
            write_sse(&mut stream, res).await?;
            return Ok(());
        }
        write_length(&mut stream, res).await?;
    }
}

/// Hop-by-hop / framing headers. axum's HeaderMap still carries
/// `Transfer-Encoding: chunked` even when we collect a finite body —
/// copying that name (by any spelling) makes Cursor 3.21.18 `terminated`.
fn keep_end_to_end(name: &str) -> bool {
    matches!(
        name,
        "content-type"
            | "cache-control"
            | "content-security-policy"
            | "x-content-type-options"
            | "x-frame-options"
            | "referrer-policy"
            | "permissions-policy"
            | "cross-origin-opener-policy"
            | "cross-origin-resource-policy"
            | "mcp-session-id"
            | "access-control-allow-origin"
            | "access-control-allow-headers"
            | "access-control-allow-methods"
            | "access-control-expose-headers"
            | "access-control-max-age"
            | "www-authenticate"
            | "retry-after"
            | "etag"
            | "last-modified"
            | "vary"
            | "location"
            | "content-disposition"
    )
}

fn push_copied(head: &mut Vec<u8>, headers: &axum::http::HeaderMap, key: HeaderName, wire: &str) {
    if let Some(value) = headers.get(&key) {
        let v = value.as_bytes();
        if v.iter().any(|b| *b == b'\r' || *b == b'\n') {
            return;
        }
        push_header(head, wire, v);
    }
}

async fn write_length(stream: &mut TcpStream, res: Response) -> std::io::Result<()> {
    let (parts, body) = res.into_parts();
    let bytes = to_bytes(body, MCP_FINITE_MAX)
        .await
        .map_err(|e| std::io::Error::new(ErrorKind::Other, e))?;
    let framed = chunk_body(&bytes);
    let mut head = Vec::with_capacity(512);
    push_status(&mut head, parts.status);
    push_header(&mut head, "transfer-encoding", b"chunked");
    push_header(&mut head, "x-gsv-framing", b"chunked-v1");
    push_copied(
        &mut head,
        &parts.headers,
        header::CONTENT_TYPE,
        "content-type",
    );
    push_copied(
        &mut head,
        &parts.headers,
        header::CACHE_CONTROL,
        "cache-control",
    );
    for key in [
        "content-security-policy",
        "x-content-type-options",
        "x-frame-options",
        "referrer-policy",
        "permissions-policy",
        "cross-origin-opener-policy",
        "cross-origin-resource-policy",
        "mcp-session-id",
        "content-encoding",
    ] {
        if let Ok(name) = HeaderName::from_bytes(key.as_bytes()) {
            push_copied(&mut head, &parts.headers, name, key);
        }
    }
    push_header(&mut head, "x-gsv-mcp", b"length");
    head.extend_from_slice(b"\r\n");
    stream.write_all(&head).await?;
    stream.write_all(&framed).await?;
    stream.flush().await
}

fn chunk_body(bytes: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(bytes.len() + 24);
    out.extend_from_slice(format!("{:x}\r\n", bytes.len()).as_bytes());
    out.extend_from_slice(bytes);
    out.extend_from_slice(b"\r\n0\r\n\r\n");
    out
}

async fn write_sse(stream: &mut TcpStream, res: Response) -> std::io::Result<()> {
    let (parts, mut body) = res.into_parts();
    let mut head = Vec::with_capacity(256);
    push_status(&mut head, parts.status);
    for (name, value) in parts.headers.iter() {
        let n = name.as_str();
        if !keep_end_to_end(n) {
            continue;
        }
        let v = value.as_bytes();
        if v.iter().any(|b| *b == b'\r' || *b == b'\n') {
            continue;
        }
        push_header(&mut head, n, v);
    }
    push_header(&mut head, "x-gsv-mcp", b"sse");
    head.extend_from_slice(b"\r\n");
    stream.write_all(&head).await?;
    stream.flush().await?;
    loop {
        match poll_data_frame(&mut body).await {
            Ok(Some(chunk)) if chunk.is_empty() => continue,
            Ok(Some(chunk)) => {
                stream.write_all(&chunk).await?;
                stream.flush().await?;
            }
            Ok(None) => return Ok(()),
            Err(e) => return Err(std::io::Error::new(ErrorKind::Other, e)),
        }
    }
}

async fn poll_data_frame(body: &mut Body) -> Result<Option<Bytes>, <Body as HttpBody>::Error> {
    match std::future::poll_fn(|cx| Pin::new(&mut *body).poll_frame(cx)).await {
        Some(Ok(frame)) => Ok(frame.into_data().ok()),
        Some(Err(e)) => Err(e),
        None => Ok(None),
    }
}

fn push_status(head: &mut Vec<u8>, status: StatusCode) {
    head.extend_from_slice(b"HTTP/1.1 ");
    head.extend_from_slice(status.as_str().as_bytes());
    head.extend_from_slice(b" OK\r\n");
}

fn push_header(head: &mut Vec<u8>, name: &str, value: &[u8]) {
    head.extend_from_slice(name.as_bytes());
    head.extend_from_slice(b": ");
    head.extend_from_slice(value);
    head.extend_from_slice(b"\r\n");
}

async fn read_request(stream: &mut TcpStream) -> std::io::Result<Option<Request<Body>>> {
    let mut buf = Vec::with_capacity(1024);
    let mut tmp = [0u8; 2048];
    let header_end = loop {
        let n = stream.read(&mut tmp).await?;
        if n == 0 {
            return if buf.is_empty() {
                Ok(None)
            } else {
                Err(std::io::Error::new(
                    ErrorKind::UnexpectedEof,
                    "eof in headers",
                ))
            };
        }
        buf.extend_from_slice(&tmp[..n]);
        if buf.len() > HEADER_MAX {
            return Err(std::io::Error::new(
                ErrorKind::InvalidData,
                "headers too large",
            ));
        }
        if let Some(i) = find_double_crlf(&buf) {
            break i;
        }
    };
    let (method, path, headers, content_length, consumed) = parse_head(&buf, header_end)?;
    let mut body = buf.split_off(consumed);
    while body.len() < content_length {
        let n = stream.read(&mut tmp).await?;
        if n == 0 {
            return Err(std::io::Error::new(ErrorKind::UnexpectedEof, "eof in body"));
        }
        body.extend_from_slice(&tmp[..n]);
        if body.len() > MCP_FINITE_MAX {
            return Err(std::io::Error::new(
                ErrorKind::InvalidData,
                "body too large",
            ));
        }
    }
    body.truncate(content_length);
    let mut builder = Request::builder().method(method).uri(path);
    for (name, value) in headers {
        builder = builder.header(name, value);
    }
    builder
        .body(Body::from(body))
        .map(Some)
        .map_err(|e| std::io::Error::new(ErrorKind::InvalidData, e))
}

fn find_double_crlf(buf: &[u8]) -> Option<usize> {
    buf.windows(4).position(|w| w == b"\r\n\r\n")
}

fn parse_head(
    buf: &[u8],
    header_end: usize,
) -> std::io::Result<(Method, String, Vec<(HeaderName, HeaderValue)>, usize, usize)> {
    let mut headers = [httparse::EMPTY_HEADER; 64];
    let mut req = httparse::Request::new(&mut headers);
    match req.parse(&buf[..header_end + 4]) {
        Ok(httparse::Status::Complete(n)) => {
            let method = req
                .method
                .ok_or_else(|| std::io::Error::new(ErrorKind::InvalidData, "method"))?
                .parse::<Method>()
                .map_err(|e| std::io::Error::new(ErrorKind::InvalidData, e))?;
            let path = req
                .path
                .ok_or_else(|| std::io::Error::new(ErrorKind::InvalidData, "path"))?
                .to_string();
            let mut out = Vec::new();
            let mut content_length = 0usize;
            for h in req.headers.iter() {
                let name = HeaderName::from_bytes(h.name.as_bytes())
                    .map_err(|e| std::io::Error::new(ErrorKind::InvalidData, e))?;
                let value = HeaderValue::from_bytes(h.value)
                    .map_err(|e| std::io::Error::new(ErrorKind::InvalidData, e))?;
                if name == header::CONTENT_LENGTH {
                    content_length = std::str::from_utf8(h.value)
                        .ok()
                        .and_then(|s| s.parse().ok())
                        .unwrap_or(0);
                }
                out.push((name, value));
            }
            Ok((method, path, out, content_length, n))
        }
        Ok(httparse::Status::Partial) => {
            Err(std::io::Error::new(ErrorKind::InvalidData, "partial"))
        }
        Err(e) => Err(std::io::Error::new(ErrorKind::InvalidData, e)),
    }
}

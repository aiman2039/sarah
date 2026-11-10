use std::io::{BufRead, Write};

use anyhow::{Context, Result, bail};
use serde_json::Value;

/// LSP `Content-Length` JSON-RPC message framing over stdio.
pub(super) struct LspFraming;

impl LspFraming {
    /// Read one framed JSON-RPC message; `Ok(None)` on clean EOF.
    pub(super) fn read_message(reader: &mut impl BufRead) -> Result<Option<Value>> {
        let Some(len) = Self::read_content_length(reader)? else {
            return Ok(None);
        };
        let mut buf = vec![0u8; len];
        reader
            .read_exact(&mut buf)
            .context("unexpected EOF in LSP body")?;
        let val: Value = serde_json::from_slice(&buf).context("invalid JSON in LSP body")?;
        Ok(Some(val))
    }

    pub(super) fn write_message(writer: &mut impl Write, payload: &Value) -> Result<()> {
        let body = serde_json::to_string(payload)?;
        write!(writer, "Content-Length: {}\r\n\r\n{body}", body.len())?;
        writer.flush()?;
        Ok(())
    }

    fn read_content_length(reader: &mut impl BufRead) -> Result<Option<usize>> {
        let mut content_len: Option<usize> = None;
        let mut line = String::new();
        loop {
            line.clear();
            let n = reader.read_line(&mut line)?;
            if n == 0 {
                return Ok(None);
            }
            let trimmed = line.trim();
            if trimmed.is_empty() {
                if let Some(len) = content_len {
                    return Ok(Some(len));
                }
                continue;
            }
            if let Some((name, val)) = trimmed.split_once(':')
                && name.trim().eq_ignore_ascii_case("content-length")
            {
                let parsed: usize = val
                    .trim()
                    .parse()
                    .context("invalid Content-Length header")?;
                content_len = Some(parsed);
            }
        }
    }

    pub(super) fn uri_to_path(uri: &str) -> Result<std::path::PathBuf> {
        let Some(rest) = uri.strip_prefix("file://") else {
            bail!("unsupported URI scheme: {uri}");
        };
        let decoded = Self::percent_decode(rest);
        #[cfg(windows)]
        let decoded = decoded.strip_prefix('/').unwrap_or(&decoded).to_string();
        Ok(std::path::PathBuf::from(decoded))
    }

    fn percent_decode(raw: &str) -> String {
        let bytes = raw.as_bytes();
        let mut out = Vec::with_capacity(bytes.len());
        let mut idx = 0usize;
        while let Some(&b) = bytes.get(idx) {
            if b == b'%'
                && let Some(hex_slice) = raw.get(idx.saturating_add(1)..idx.saturating_add(3))
                && let Ok(decoded) = u8::from_str_radix(hex_slice, 16)
            {
                out.push(decoded);
                idx = idx.saturating_add(3);
                continue;
            }
            out.push(b);
            idx = idx.saturating_add(1);
        }
        String::from_utf8_lossy(&out).into_owned()
    }
}

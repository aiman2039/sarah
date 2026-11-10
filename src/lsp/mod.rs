//! Language Server Protocol (stdio): live diagnostics on open, change and save.

mod diagnostics;
mod framing;

use std::io::{BufReader, Write};
use std::path::PathBuf;
use std::process::ExitCode;

use anyhow::Result;
use serde_json::{Value, json};

use self::diagnostics::LspDiagnostics;
use self::framing::LspFraming;

#[derive(Default)]
pub struct LspServer {
    root: Option<PathBuf>,
}

impl LspServer {
    pub fn run_stdio() -> Result<ExitCode> {
        let stdin = std::io::stdin();
        let stdout = std::io::stdout();
        let mut reader = BufReader::new(stdin.lock());
        let mut writer = stdout.lock();
        let mut server = Self::default();
        while let Some(msg) = LspFraming::read_message(&mut reader)? {
            if server.handle(&msg, &mut writer)? {
                break;
            }
        }
        Ok(ExitCode::SUCCESS)
    }

    /// Returns `true` when `exit` was received.
    fn handle(&mut self, msg: &Value, writer: &mut impl Write) -> Result<bool> {
        let Some(method) = msg.get("method").and_then(Value::as_str) else {
            return Ok(false);
        };
        match method {
            "initialize" => self.on_initialize(msg, writer)?,
            "shutdown" => Self::reply(writer, msg.get("id"), json!(null))?,
            "exit" => return Ok(true),
            "textDocument/didOpen" => self.on_did_open(msg, writer)?,
            "textDocument/didChange" => self.on_did_change(msg, writer)?,
            "textDocument/didSave" => self.on_did_save(msg, writer)?,
            "textDocument/didClose" => Self::on_did_close(msg, writer)?,
            _ => {}
        }
        Ok(false)
    }

    fn on_initialize(&mut self, msg: &Value, writer: &mut impl Write) -> Result<()> {
        if let Some(uri) = msg
            .pointer("/params/rootUri")
            .and_then(Value::as_str)
            .or_else(|| {
                msg.pointer("/params/workspaceFolders/0/uri")
                    .and_then(Value::as_str)
            })
            && let Ok(path) = LspFraming::uri_to_path(uri)
        {
            self.root = Some(path);
        }
        Self::reply(
            writer,
            msg.get("id"),
            json!({
                "capabilities": {
                    "textDocumentSync": {
                        "openClose": true,
                        "change": 1,
                        "save": { "includeText": true }
                    }
                },
                "serverInfo": {
                    "name": "sarah",
                    "version": env!("CARGO_PKG_VERSION")
                }
            }),
        )
    }

    fn on_did_open(&self, msg: &Value, writer: &mut impl Write) -> Result<()> {
        let Some(uri) = msg
            .pointer("/params/textDocument/uri")
            .and_then(Value::as_str)
        else {
            return Ok(());
        };
        let text = msg
            .pointer("/params/textDocument/text")
            .and_then(Value::as_str);
        self.publish(writer, uri, text)
    }

    fn on_did_change(&self, msg: &Value, writer: &mut impl Write) -> Result<()> {
        let Some(uri) = msg
            .pointer("/params/textDocument/uri")
            .and_then(Value::as_str)
        else {
            return Ok(());
        };
        let text = msg
            .pointer("/params/contentChanges")
            .and_then(Value::as_array)
            .and_then(|arr| arr.last())
            .and_then(|c| c.get("text"))
            .and_then(Value::as_str);
        self.publish(writer, uri, text)
    }

    fn on_did_save(&self, msg: &Value, writer: &mut impl Write) -> Result<()> {
        let Some(uri) = msg
            .pointer("/params/textDocument/uri")
            .and_then(Value::as_str)
        else {
            return Ok(());
        };
        let text = msg.pointer("/params/text").and_then(Value::as_str);
        self.publish(writer, uri, text)
    }

    fn on_did_close(msg: &Value, writer: &mut impl Write) -> Result<()> {
        let Some(uri) = msg
            .pointer("/params/textDocument/uri")
            .and_then(Value::as_str)
        else {
            return Ok(());
        };
        LspFraming::write_message(
            writer,
            &json!({
                "jsonrpc": "2.0",
                "method": "textDocument/publishDiagnostics",
                "params": { "uri": uri, "diagnostics": [] }
            }),
        )
    }

    fn publish(&self, writer: &mut impl Write, uri: &str, text: Option<&str>) -> Result<()> {
        let diagnostics = LspDiagnostics::for_document(self.root.as_deref(), uri, text);
        LspFraming::write_message(
            writer,
            &json!({
                "jsonrpc": "2.0",
                "method": "textDocument/publishDiagnostics",
                "params": { "uri": uri, "diagnostics": diagnostics }
            }),
        )
    }

    fn reply(writer: &mut impl Write, id: Option<&Value>, result: Value) -> Result<()> {
        let Some(id) = id else { return Ok(()) };
        LspFraming::write_message(
            writer,
            &json!({ "jsonrpc": "2.0", "id": id, "result": result }),
        )
    }
}

#![expect(clippy::expect_used)]
use crate::error::TldError;
use serde_json::{Value, json};
use std::collections::HashMap;
use std::process::Stdio;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, Command};
use tokio::sync::{mpsc, oneshot};

enum OutboundMessage {
    Request(Value, oneshot::Sender<Value>),
    Notification(Value),
}

pub struct Session {
    _child: Child,
    message_tx: mpsc::Sender<OutboundMessage>,
}

impl Session {
    pub fn start(executable: &str, args: &[String], root_dir: &str) -> Result<Self, TldError> {
        let mut child = Command::new(executable)
            .args(args)
            .current_dir(root_dir)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| TldError::Generic(format!("Failed to start LSP {executable}: {e}")))?;

        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| TldError::Generic("No stdin".to_string()))?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| TldError::Generic("No stdout".to_string()))?;

        let (message_tx, mut message_rx) = mpsc::channel::<OutboundMessage>(100);

        tokio::spawn(async move {
            let mut stdin = stdin;
            let mut stdout_reader = BufReader::new(stdout);
            let mut pending_requests: HashMap<i64, oneshot::Sender<Value>> = HashMap::new();
            let mut next_id: i64 = 1;

            loop {
                tokio::select! {
                    Some(message) = message_rx.recv() => {
                        let mut payload = match message {
                            OutboundMessage::Request(mut params, response_tx) => {
                                let id = next_id;
                                next_id += 1;
                                params["id"] = json!(id);
                                pending_requests.insert(id, response_tx);
                                params
                            }
                            OutboundMessage::Notification(params) => params,
                        };

                        payload["jsonrpc"] = json!("2.0");

                        let body = serde_json::to_string(&payload)
                            .expect("LSP params must be serializable");
                        let msg = format!("Content-Length: {}\r\n\r\n{}", body.len(), body);
                        if stdin.write_all(msg.as_bytes()).await.is_err() {
                            break;
                        }
                    }
                    line = read_lsp_message(&mut stdout_reader) => {
                        match line {
                            Ok(Some(content)) => {
                                if let Ok(msg) = serde_json::from_str::<Value>(&content)
                                    && let Some(id) = msg.get("id").and_then(Value::as_i64)
                                    && let Some(tx) = pending_requests.remove(&id)
                                {
                                    let _ = tx.send(msg);
                                }
                            }
                            _ => break,
                        }
                    }
                }
            }
        });

        Ok(Session {
            _child: child,
            message_tx,
        })
    }

    pub async fn initialize(&self, root_uri: String) -> Result<(), TldError> {
        let params = json!({
            "method": "initialize",
            "params": {
                "processId": std::process::id(),
                "rootUri": root_uri,
                "capabilities": {},
                "workspaceFolders": [{"uri": root_uri, "name": "root"}]
            }
        });
        self.send_request(params).await?;
        self.send_notification(json!({
            "method": "initialized",
            "params": {}
        }))
        .await
    }

    pub async fn send_request(&self, params: Value) -> Result<Value, TldError> {
        let (tx, rx) = oneshot::channel();
        self.message_tx
            .send(OutboundMessage::Request(params, tx))
            .await
            .map_err(|_| TldError::Generic("LSP channel closed".to_string()))?;
        rx.await
            .map_err(|_| TldError::Generic("LSP response dropped".to_string()))
    }

    pub async fn send_notification(&self, params: Value) -> Result<(), TldError> {
        self.message_tx
            .send(OutboundMessage::Notification(params))
            .await
            .map_err(|_| TldError::Generic("LSP channel closed".to_string()))
    }

    pub async fn did_open(
        &self,
        file_uri: &str,
        language_id: &str,
        text: &str,
    ) -> Result<(), TldError> {
        self.send_notification(json!({
            "method": "textDocument/didOpen",
            "params": {
                "textDocument": {
                    "uri": file_uri,
                    "languageId": language_id,
                    "version": 1,
                    "text": text,
                }
            }
        }))
        .await
    }

    pub async fn did_close(&self, file_uri: &str) -> Result<(), TldError> {
        self.send_notification(json!({
            "method": "textDocument/didClose",
            "params": {
                "textDocument": {
                    "uri": file_uri,
                }
            }
        }))
        .await
    }

    pub async fn definition(
        &self,
        file_uri: &str,
        line: i32,
        column: i32,
    ) -> Result<Value, TldError> {
        self.send_request(json!({
            "method": "textDocument/definition",
            "params": {
                "textDocument": {"uri": file_uri},
                "position": {"line": line - 1, "character": column - 1}
            }
        }))
        .await
    }

    pub async fn implementation(
        &self,
        file_uri: &str,
        line: i32,
        column: i32,
    ) -> Result<Value, TldError> {
        self.send_request(json!({
            "method": "textDocument/implementation",
            "params": {
                "textDocument": {"uri": file_uri},
                "position": {"line": line - 1, "character": column - 1}
            }
        }))
        .await
    }

    pub async fn document_symbols(&self, file_uri: &str) -> Result<Value, TldError> {
        self.send_request(json!({
            "method": "textDocument/documentSymbol",
            "params": {
                "textDocument": {"uri": file_uri}
            }
        }))
        .await
    }

    pub async fn shutdown(&self) -> Result<(), TldError> {
        let _ = self
            .send_request(json!({"method": "shutdown", "params": null}))
            .await;
        let _ = self
            .send_notification(json!({"method": "exit", "params": null}))
            .await;
        Ok(())
    }
}

async fn read_lsp_message(
    reader: &mut BufReader<tokio::process::ChildStdout>,
) -> Result<Option<String>, std::io::Error> {
    let mut line = String::new();
    let mut content_length: usize = 0;

    loop {
        line.clear();
        if reader.read_line(&mut line).await? == 0 {
            return Ok(None);
        }
        if line == "\r\n" {
            break;
        }
        if let Some(rest) = line.strip_prefix("Content-Length: ") {
            content_length = rest.trim().parse::<usize>().unwrap_or(0);
        }
    }

    if content_length == 0 {
        return Ok(None);
    }

    let mut body = vec![0u8; content_length];
    reader.read_exact(&mut body).await?;
    Ok(Some(String::from_utf8_lossy(&body).to_string()))
}

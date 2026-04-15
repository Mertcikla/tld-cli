use std::collections::HashMap;
use std::process::Stdio;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, Command};
use tokio::sync::{mpsc, oneshot};
use lsp_types::*;
use serde_json::{json, Value};
use crate::error::TldError;

pub struct Session {
    child: Child,
    request_tx: mpsc::Sender<(Value, oneshot::Sender<Value>)>,
}

impl Session {
    pub async fn start(executable: &str, args: &[String], root_dir: &str) -> Result<Self, TldError> {
        let mut child = Command::new(executable)
            .args(args)
            .current_dir(root_dir)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| TldError::Generic(format!("Failed to start LSP {}: {}", executable, e)))?;

        let stdin = child.stdin.take().ok_or_else(|| TldError::Generic("No stdin".to_string()))?;
        let stdout = child.stdout.take().ok_or_else(|| TldError::Generic("No stdout".to_string()))?;

        let (request_tx, mut request_rx) = mpsc::channel::<(Value, oneshot::Sender<Value>)>(100);

        // Spawn IO loops
        tokio::spawn(async move {
            let mut stdin = stdin;
            let mut stdout_reader = BufReader::new(stdout);
            let mut pending_requests = HashMap::new();
            let mut next_id = 1;

            loop {
                tokio::select! {
                    Some((mut method_params, response_tx)) = request_rx.recv() => {
                        let id = next_id;
                        next_id += 1;
                        method_params["id"] = json!(id);
                        method_params["jsonrpc"] = json!("2.0");
                        
                        let body = serde_json::to_string(&method_params).unwrap();
                        let msg = format!("Content-Length: {}\r\n\r\n{}", body.len(), body);
                        if stdin.write_all(msg.as_bytes()).await.is_err() {
                            break;
                        }
                        pending_requests.insert(id, response_tx);
                    }
                    line = read_line(&mut stdout_reader) => {
                        if let Ok(Some(content)) = line {
                            if let Ok(msg) = serde_json::from_str::<Value>(&content) {
                                if let Some(id) = msg.get("id").and_then(|v| v.as_i64()) {
                                    if let Some(tx) = pending_requests.remove(&(id as i32)) {
                                        let _ = tx.send(msg);
                                    }
                                }
                            }
                        } else {
                            break;
                        }
                    }
                }
            }
        });

        let session = Session {
            child,
            request_tx,
        };

        Ok(session)
    }

    pub async fn initialize(&self, root_uri: Url) -> Result<(), TldError> {
        let params = json!({
            "method": "initialize",
            "params": InitializeParams {
                process_id: Some(std::process::id()),
                root_uri: Some(root_uri.clone()),
                root_path: None,
                initialization_options: None,
                capabilities: ClientCapabilities::default(),
                trace: None,
                workspace_folders: Some(vec![WorkspaceFolder {
                    uri: root_uri,
                    name: "root".to_string(),
                }]),
                client_info: None,
                locale: None,
            }
        });

        self.send_request(params).await?;
        Ok(())
    }

    async fn send_request(&self, params: Value) -> Result<Value, TldError> {
        let (tx, rx) = oneshot::channel();
        self.request_tx.send((params, tx)).await
            .map_err(|_| TldError::Generic("LSP channel closed".to_string()))?;
        
        rx.await.map_err(|_| TldError::Generic("LSP response dropped".to_string()))
    }
}

async fn read_line(reader: &mut BufReader<tokio::process::ChildStdout>) -> Result<Option<String>, std::io::Error> {
    let mut line = String::new();
    let mut content_length = 0;

    // Read headers
    loop {
        line.clear();
        if reader.read_line(&mut line).await? == 0 {
            return Ok(None);
        }
        if line == "\r\n" {
            break;
        }
        if line.starts_with("Content-Length: ") {
            content_length = line["Content-Length: ".len()..]
                .trim()
                .parse::<usize>()
                .unwrap_or(0);
        }
    }

    if content_length == 0 {
        return Ok(None);
    }

    let mut body = vec![0u8; content_length];
    reader.read_exact(&mut body).await?;
    Ok(Some(String::from_utf8_lossy(&body).to_string()))
}

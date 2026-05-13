use anyhow::Result;
use tokio::process::{Child, Command};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::sync::mpsc;
use std::process::Stdio;

pub enum TransportEvent {
    Message(String),
    Error(String),
    Closed,
}

pub struct StdioTransport {
    child: Child,
    reader_tx: mpsc::UnboundedSender<TransportEvent>,
}

impl StdioTransport {
    pub async fn connect(command: &str, args: &[String]) -> Result<(Self, mpsc::UnboundedReceiver<TransportEvent>)> {
        let (tx, rx) = mpsc::unbounded_channel();
        let reader_tx = tx.clone();
        let tx_for_stderr = tx.clone();

        let mut child = Command::new(command)
            .args(args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;

        let stdout = child.stdout.take()
            .ok_or_else(|| anyhow::anyhow!("Failed to capture stdout"))?;
        let stderr = child.stderr.take()
            .ok_or_else(|| anyhow::anyhow!("Failed to capture stderr"))?;

        tokio::spawn(async move {
            let mut reader = BufReader::new(stdout);
            let mut line = String::new();
            loop {
                line.clear();
                match reader.read_line(&mut line).await {
                    Ok(0) => {
                        let _ = tx.send(TransportEvent::Closed);
                        break;
                    }
                    Ok(_) => {
                        let trimmed = line.trim().to_string();
                        if !trimmed.is_empty() {
                            let _ = tx.send(TransportEvent::Message(trimmed));
                        }
                    }
                    Err(e) => {
                        let _ = tx.send(TransportEvent::Error(e.to_string()));
                        break;
                    }
                }
            }
        });

        tokio::spawn(async move {
            let mut reader = BufReader::new(stderr);
            let mut line = String::new();
            loop {
                line.clear();
                match reader.read_line(&mut line).await {
                    Ok(0) => break,
                    Ok(_) => {
                        let trimmed = line.trim().to_string();
                        if !trimmed.is_empty() {
                            let _ = tx_for_stderr.send(TransportEvent::Error(trimmed));
                        }
                    }
                    Err(_) => break,
                }
            }
        });

        Ok((Self { child, reader_tx }, rx))
    }

    pub async fn send(&mut self, message: &str) -> Result<()> {
        if let Some(stdin) = self.child.stdin.as_mut() {
            stdin.write_all(message.as_bytes()).await?;
            stdin.write_all(b"\n").await?;
            stdin.flush().await?;
            Ok(())
        } else {
            Err(anyhow::anyhow!("Stdin not available"))
        }
    }

    pub async fn shutdown(&mut self) -> Result<()> {
        self.child.kill().await?;
        self.child.wait().await?;
        Ok(())
    }
}

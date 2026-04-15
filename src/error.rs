/// Structured error type for the tld CLI.
use std::fmt;

#[derive(Debug)]
pub enum TldError {
    Io(std::io::Error),
    Yaml(String),
    Grpc(tonic::Status),
    Transport(tonic::transport::Error),
    Auth(String),
    Workspace(String),
    Generic(String),
}

impl fmt::Display for TldError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TldError::Io(e) => write!(f, "IO error: {}", e),
            TldError::Yaml(s) => write!(f, "YAML error: {}", s),
            TldError::Grpc(s) => write!(f, "gRPC error: {} ({})", s.message(), s.code()),
            TldError::Transport(e) => write!(f, "Transport error: {}", e),
            TldError::Auth(s) => write!(
                f,
                "Authentication error: {}\n\nRun `tld login` to authenticate.",
                s
            ),
            TldError::Workspace(s) => write!(f, "Workspace error: {}", s),
            TldError::Generic(s) => write!(f, "{}", s),
        }
    }
}

impl std::error::Error for TldError {}

impl From<std::io::Error> for TldError {
    fn from(e: std::io::Error) -> Self {
        TldError::Io(e)
    }
}

impl From<tonic::Status> for TldError {
    fn from(s: tonic::Status) -> Self {
        if s.code() == tonic::Code::Unauthenticated {
            TldError::Auth(s.message().to_string())
        } else {
            TldError::Grpc(s)
        }
    }
}

impl From<tonic::transport::Error> for TldError {
    fn from(e: tonic::transport::Error) -> Self {
        TldError::Transport(e)
    }
}

impl From<String> for TldError {
    fn from(s: String) -> Self {
        TldError::Generic(s)
    }
}

impl From<&str> for TldError {
    fn from(s: &str) -> Self {
        TldError::Generic(s.to_string())
    }
}

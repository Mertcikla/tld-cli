#![allow(dead_code)]
//! HTTP endpoint recognition from framework annotations / decorators.
//!
//! Language-agnostic: the registry below matches annotation *names* (and in a
//! few cases, the receiver prefix like `router.`) rather than per-language
//! AST shapes. Every parser produces `Annotation { name, args }` values in the
//! same shape, so a single table serves Python, TypeScript, Java, and Rust.

use super::types::{SemanticBundle, SymbolId};
use crate::analyzer::types::Annotation;
use std::collections::HashMap;

/// Canonical HTTP methods we care about for role inference.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HttpMethod {
    Get,
    Post,
    Put,
    Delete,
    Patch,
    Head,
    Options,
    Any,
}

impl HttpMethod {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Get => "GET",
            Self::Post => "POST",
            Self::Put => "PUT",
            Self::Delete => "DELETE",
            Self::Patch => "PATCH",
            Self::Head => "HEAD",
            Self::Options => "OPTIONS",
            Self::Any => "ANY",
        }
    }

    fn from_lowercase(token: &str) -> Option<Self> {
        match token {
            "get" => Some(Self::Get),
            "post" => Some(Self::Post),
            "put" => Some(Self::Put),
            "delete" => Some(Self::Delete),
            "patch" => Some(Self::Patch),
            "head" => Some(Self::Head),
            "options" => Some(Self::Options),
            "any" | "all" | "request" | "route" => Some(Self::Any),
            _ => None,
        }
    }
}

/// An HTTP endpoint stamp derived from an annotation.
#[derive(Debug, Clone)]
pub struct HttpEndpoint {
    pub method: HttpMethod,
    /// Path template if the annotation exposes one; empty when unavailable.
    pub path: String,
    /// Source annotation name (for debugging).
    pub source_annotation: String,
}

/// Class-level framework roles ("controller-ness"). A class stamped as a
/// controller propagates an implicit endpoint role to its handler methods if
/// they are public.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ControllerKind {
    /// Spring @RestController / @Controller, NestJS @Controller, DRF APIView.
    Controller,
    /// DRF @api_view decorator on a function.
    ApiView,
}

/// Check whether an annotation identifies an HTTP handler directly.
pub fn detect_endpoint(annotation: &Annotation) -> Option<HttpEndpoint> {
    let raw = annotation.name.trim();
    if raw.is_empty() {
        return None;
    }

    // Strip common prefixes ("@", "::"), then take the final segment after
    // `.` or `::` so `router.get`, `app.get`, `actix_web::get`, `fastapi.APIRouter.get`
    // all reduce to `get`. We remember the full form for disambiguation.
    let stripped = raw.trim_start_matches('@');
    let lower = stripped.to_ascii_lowercase();
    let last_dot = lower
        .rsplit_once('.')
        .map(|(_, tail)| tail)
        .unwrap_or(&lower);
    let last_seg = last_dot
        .rsplit_once("::")
        .map(|(_, tail)| tail)
        .unwrap_or(last_dot);

    // Case 1: direct verb annotation (e.g. "get", "post", "GetMapping").
    if let Some(method) = direct_verb(last_seg) {
        let path = first_string_arg(&annotation.args);
        return Some(HttpEndpoint {
            method,
            path,
            source_annotation: raw.to_string(),
        });
    }

    // Case 2: Spring-style verb-prefixed camelcase ("getmapping" → GET).
    if let Some(method) = spring_mapping(last_seg) {
        let path = first_string_arg(&annotation.args);
        return Some(HttpEndpoint {
            method,
            path,
            source_annotation: raw.to_string(),
        });
    }

    // Case 3: generic "route" / "requestmapping" — method unknown.
    if matches!(
        last_seg,
        "route" | "requestmapping" | "request_mapping" | "api_view" | "action"
    ) {
        return Some(HttpEndpoint {
            method: HttpMethod::Any,
            path: first_string_arg(&annotation.args),
            source_annotation: raw.to_string(),
        });
    }

    None
}

/// Detect class-level framework controller markers.
pub fn detect_controller(annotation: &Annotation) -> Option<ControllerKind> {
    let raw = annotation.name.trim().trim_start_matches('@');
    let last = raw
        .rsplit_once('.')
        .map(|(_, t)| t)
        .unwrap_or(raw)
        .rsplit_once("::")
        .map(|(_, t)| t)
        .unwrap_or_else(|| {
            raw.rsplit_once('.')
                .map(|(_, t)| t)
                .unwrap_or(raw)
        });
    match last {
        "RestController" | "Controller" => Some(ControllerKind::Controller),
        "api_view" => Some(ControllerKind::ApiView),
        _ => None,
    }
}

/// Scan a SemanticBundle and stamp every symbol whose annotations identify an
/// HTTP handler. Returns a map of symbol_id → endpoint.
pub fn detect_endpoints(bundle: &SemanticBundle) -> HashMap<SymbolId, HttpEndpoint> {
    let mut out = HashMap::new();
    for sym in &bundle.symbols {
        for ann in &sym.annotations {
            if let Some(ep) = detect_endpoint(ann) {
                out.insert(sym.symbol_id.clone(), ep);
                break;
            }
        }
    }
    out
}

/// Route-call recognition for frameworks that wire handlers via builder calls
/// (gin `router.GET("/x", handler)`, express `app.get(...)`, actix
/// `.route("/x", web::get().to(handler))`). Given the receiver expression
/// and the method name at the call site, return the HTTP method when the
/// pattern looks like a route registration.
pub fn detect_route_call(receiver: &str, method_name: &str) -> Option<HttpMethod> {
    let receiver_lower = receiver.trim().to_ascii_lowercase();
    if receiver_lower.is_empty() {
        return None;
    }
    // Receiver heuristics: common router / mux / group / app names.
    let looks_like_router = ROUTER_RECEIVERS
        .iter()
        .any(|r| receiver_lower == *r || receiver_lower.ends_with(r));
    if !looks_like_router {
        return None;
    }
    HttpMethod::from_lowercase(&method_name.to_ascii_lowercase())
}

const ROUTER_RECEIVERS: &[&str] = &[
    "router", "app", "group", "r", "e", "mux", "api", "srv", "server", "engine", "handler", "rg",
    "bp", "blueprint",
];

fn direct_verb(s: &str) -> Option<HttpMethod> {
    HttpMethod::from_lowercase(s)
}

fn spring_mapping(s: &str) -> Option<HttpMethod> {
    match s {
        "getmapping" => Some(HttpMethod::Get),
        "postmapping" => Some(HttpMethod::Post),
        "putmapping" => Some(HttpMethod::Put),
        "deletemapping" => Some(HttpMethod::Delete),
        "patchmapping" => Some(HttpMethod::Patch),
        _ => None,
    }
}

/// Extract the first string-literal argument, stripping surrounding quotes.
fn first_string_arg(args: &[String]) -> String {
    for arg in args {
        let trimmed = arg.trim();
        // Skip named parameters like `method = "GET"`.
        if trimmed.contains('=') && !trimmed.starts_with('"') && !trimmed.starts_with('\'') {
            continue;
        }
        let unquoted = trimmed
            .trim_start_matches('"')
            .trim_end_matches('"')
            .trim_start_matches('\'')
            .trim_end_matches('\'');
        if !unquoted.is_empty() {
            return unquoted.to_string();
        }
    }
    String::new()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ann(name: &str, args: &[&str]) -> Annotation {
        Annotation {
            name: name.to_string(),
            args: args.iter().map(|s| (*s).to_string()).collect(),
        }
    }

    #[test]
    fn recognizes_fastapi_router_get() {
        let ep = detect_endpoint(&ann("router.get", &["\"/articles\""])).unwrap();
        assert_eq!(ep.method, HttpMethod::Get);
        assert_eq!(ep.path, "/articles");
    }

    #[test]
    fn recognizes_spring_mapping_variants() {
        let ep = detect_endpoint(&ann("GetMapping", &["\"/users\""])).unwrap();
        assert_eq!(ep.method, HttpMethod::Get);
        assert_eq!(ep.path, "/users");
        let ep = detect_endpoint(&ann("PostMapping", &["\"/users\""])).unwrap();
        assert_eq!(ep.method, HttpMethod::Post);
    }

    #[test]
    fn recognizes_rocket_attribute() {
        let ep = detect_endpoint(&ann("post", &["\"/articles\""])).unwrap();
        assert_eq!(ep.method, HttpMethod::Post);
        assert_eq!(ep.path, "/articles");
    }

    #[test]
    fn recognizes_actix_scoped_path() {
        let ep = detect_endpoint(&ann("actix_web::get", &["\"/users/<id>\""])).unwrap();
        assert_eq!(ep.method, HttpMethod::Get);
        assert_eq!(ep.path, "/users/<id>");
    }

    #[test]
    fn recognizes_flask_route() {
        let ep = detect_endpoint(&ann("app.route", &["\"/login\""])).unwrap();
        assert_eq!(ep.method, HttpMethod::Any);
        assert_eq!(ep.path, "/login");
    }

    #[test]
    fn ignores_non_endpoint_annotations() {
        assert!(detect_endpoint(&ann("staticmethod", &[])).is_none());
        assert!(detect_endpoint(&ann("Override", &[])).is_none());
        assert!(detect_endpoint(&ann("derive", &["Debug", "Clone"])).is_none());
    }

    #[test]
    fn detects_controller_marker() {
        assert_eq!(
            detect_controller(&ann("RestController", &[])),
            Some(ControllerKind::Controller)
        );
        assert_eq!(
            detect_controller(&ann("@Controller", &[])),
            Some(ControllerKind::Controller)
        );
        assert_eq!(
            detect_controller(&ann("api_view", &["[\"GET\"]"])),
            Some(ControllerKind::ApiView)
        );
        assert!(detect_controller(&ann("Component", &[])).is_none());
    }

    #[test]
    fn route_call_matches_gin_style() {
        assert_eq!(detect_route_call("router", "GET"), Some(HttpMethod::Get));
        assert_eq!(
            detect_route_call("apiGroup", "POST"),
            Some(HttpMethod::Post)
        );
        assert!(detect_route_call("foo", "GET").is_none());
    }
}

macro_rules! query_source {
    ($path:literal) => {
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/src/analyzer/queries/",
            $path
        ))
    };
}

pub fn cpp_declarations() -> &'static str {
    query_source!("cpp/declarations.scm")
}

pub fn cpp_members() -> &'static str {
    query_source!("cpp/members.scm")
}

pub fn cpp_references() -> &'static str {
    query_source!("cpp/references.scm")
}

pub fn go_declarations() -> &'static str {
    query_source!("go/declarations.scm")
}

pub fn go_references() -> &'static str {
    query_source!("go/references.scm")
}

pub fn go_control() -> &'static str {
    query_source!("go/control.scm")
}

pub fn java_declarations() -> &'static str {
    query_source!("java/declarations.scm")
}

pub fn java_class_members() -> &'static str {
    query_source!("java/class_members.scm")
}

pub fn java_interface_members() -> &'static str {
    query_source!("java/interface_members.scm")
}

pub fn java_references() -> &'static str {
    query_source!("java/references.scm")
}

pub fn java_control() -> &'static str {
    query_source!("java/control.scm")
}

pub fn python_declarations() -> &'static str {
    query_source!("python/declarations.scm")
}

pub fn python_references() -> &'static str {
    query_source!("python/references.scm")
}

pub fn python_members() -> &'static str {
    query_source!("python/members.scm")
}

pub fn python_control() -> &'static str {
    query_source!("python/control.scm")
}

pub fn rust_declarations() -> &'static str {
    query_source!("rust/declarations.scm")
}

pub fn rust_impl_members() -> &'static str {
    query_source!("rust/impl_members.scm")
}

pub fn rust_references() -> &'static str {
    query_source!("rust/references.scm")
}

pub fn typescript_declarations() -> &'static str {
    query_source!("typescript/declarations.scm")
}

pub fn typescript_members() -> &'static str {
    query_source!("typescript/members.scm")
}

pub fn typescript_references() -> &'static str {
    query_source!("typescript/references.scm")
}

pub fn typescript_control() -> &'static str {
    query_source!("typescript/control.scm")
}

pub fn javascript_declarations() -> &'static str {
    query_source!("javascript/declarations.scm")
}

pub fn javascript_members() -> &'static str {
    query_source!("javascript/members.scm")
}

pub fn javascript_references() -> &'static str {
    query_source!("javascript/references.scm")
}

pub fn javascript_control() -> &'static str {
    query_source!("javascript/control.scm")
}

#[cfg(test)]
mod tests {
    use super::*;
    use tree_sitter::Query;
    use ts_pack_core::get_language;

    fn assert_compiles(language_name: &str, query_source: &str) {
        let language = get_language(language_name)
            .unwrap_or_else(|err| panic!("failed to load {language_name} language: {err}"));
        Query::new(&language, query_source)
            .unwrap_or_else(|err| panic!("failed to compile {language_name} query: {err}"));
    }

    #[test]
    fn go_queries_compile() {
        assert_compiles("go", go_declarations());
        assert_compiles("go", go_references());
        assert_compiles("go", go_control());
    }

    #[test]
    fn python_queries_compile() {
        assert_compiles("python", python_declarations());
        assert_compiles("python", python_references());
        assert_compiles("python", python_members());
        assert_compiles("python", python_control());
    }

    #[test]
    fn java_queries_compile() {
        assert_compiles("java", java_declarations());
        assert_compiles("java", java_class_members());
        assert_compiles("java", java_interface_members());
        assert_compiles("java", java_references());
        assert_compiles("java", java_control());
    }

    #[test]
    fn rust_queries_compile() {
        assert_compiles("rust", rust_declarations());
        assert_compiles("rust", rust_impl_members());
        assert_compiles("rust", rust_references());
    }

    #[test]
    fn cpp_queries_compile() {
        assert_compiles("cpp", cpp_declarations());
        assert_compiles("cpp", cpp_members());
        assert_compiles("cpp", cpp_references());
    }

    #[test]
    fn typescript_queries_compile() {
        assert_compiles("typescript", typescript_declarations());
        assert_compiles("typescript", typescript_members());
        assert_compiles("typescript", typescript_references());
        assert_compiles("typescript", typescript_control());
    }

    #[test]
    fn javascript_queries_compile() {
        assert_compiles("javascript", javascript_declarations());
        assert_compiles("javascript", javascript_members());
        assert_compiles("javascript", javascript_references());
        assert_compiles("javascript", javascript_control());
    }
}

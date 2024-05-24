use infrastructure::supported_languages::SupportedLanguages;

#[derive(Debug, Clone)]
pub struct Codebase {
    pub name: String,
    pub url: String,
    pub lang: SupportedLanguages,
    pub working_dir: Option<String>,
    pub test_command: Option<String>,
    pub coverage_command: Option<String>,
}

impl Codebase {
    pub fn supported_extensions(&self) -> &[&str] {
        self.lang.file_extensions()
    }

    /// A human readable unique identifier for a codebase
    ///
    /// I.e. bosun-ai/fluyt/rust
    ///
    /// Used for cache prefixes and storage namespacing
    ///
    /// @example
    /// ```
    /// use workspace::Codebase;
    /// use infrastructure::SupportedLanguages;
    ///
    /// let codebase = Codebase {
    ///    name: "bosun-ai/fluyt".to_string(),
    ///    url: "https://github.com/bosun-ai/fluyt".to_string(),
    ///    lang: SupportedLanguages::Rust,
    ///    working_dir: None,
    ///    test_command: None,
    ///    coverage_command: None,
    ///  };
    ///  assert_eq!(&codebase.huuid(), "bosun-ai-fluyt");
    ///
    ///  let codebase_with_workdir = Codebase {
    ///    working_dir: Some("src".to_string()),
    ///    ..codebase.clone()
    ///  };
    ///  assert_eq!(&codebase_with_workdir.huuid(), "bosun-ai-fluyt-src");
    /// ```
    pub fn huuid(&self) -> String {
        if cfg!(feature = "integration_testing") {
            return format!("test-{}-{}", self.name.replace(['/', ':'], "-"), self.lang);
        }
        format!(
            "{}-{}",
            self.name.replace(['/', ':'], "-"),
            self.working_dir.as_deref().unwrap_or("")
        )
        .trim_end_matches('-')
        .to_string()
    }
}

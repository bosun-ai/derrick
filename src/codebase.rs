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
}

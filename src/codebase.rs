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

static RUST_EXTENSIONS: &[&str] = &["rs"];
static TYPESCRIPT_EXTENSIONS: &[&str] = &["ts"];

impl Codebase {
    pub fn supported_extensions(&self) -> &[&str] {
        match self.lang {
            SupportedLanguages::Rust => RUST_EXTENSIONS,
            SupportedLanguages::Typescript => TYPESCRIPT_EXTENSIONS,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Codebase {
    pub name: String,
    pub url: String,
    pub lang: String,
    pub working_dir: Option<String>,
    pub test_command: Option<String>,
    pub coverage_command: Option<String>,
}

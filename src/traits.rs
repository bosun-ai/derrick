use anyhow::Result;
use async_trait::async_trait;
#[async_trait]
pub trait Workspace {
    async fn exec_cmd(&self, cmd: &Command) -> Result<CommandOutput>;

    async fn init(&self) -> Result<()>;

    async fn teardown(self) -> Result<()>;
}

// CommandOutput is just an alias for String
pub type CommandOutput = String;

// Implementors decide what they support, i.e. local might never want to support unsaferaw
#[non_exhaustive]
pub enum Command {
    Git(GitCommands),
    Github(GithubCommands),
    File(FileCommands),
    Code(CodeCommands),
    UnsafeRaw(String),
}

#[non_exhaustive]
pub enum GitCommands {
    Clone { url: String },
    Checkout { branch: String },
    Commit { commit_message: String },
    Reset,
    Push,
}

#[non_exhaustive]
pub enum GithubCommands {
    CreatePullRequest { title: String, body: String },
}

#[non_exhaustive]
pub enum FileCommands {
    Read { filename: String },
    Write { filename: String, body: String },
}

#[non_exhaustive]
pub enum CodeCommands {
    Search { query: String },
    RunTests,
}

impl Into<Command> for CodeCommands {
    fn into(self) -> Command {
        Command::Code(self)
    }
}

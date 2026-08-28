use std::error::Error;
use std::process::{Command, Output};

use tempfile::TempDir;

pub struct TestCli {
    data_dir: TempDir,
}

impl TestCli {
    pub fn new() -> Result<Self, Box<dyn Error>> {
        Ok(Self { data_dir: tempfile::tempdir()? })
    }

    pub fn run(&self, gateway_url: &str, args: &[&str]) -> Result<Output, Box<dyn Error>> {
        let output = Command::new(env!("CARGO_BIN_EXE_grokctl"))
            .args(args)
            .args(["--gateway-url", gateway_url])
            .args(["--gateway-token", "test-token"])
            .args(["--format", "json"])
            .env("GROKCTL_DATA_DIR", self.data_dir.path())
            .output()?;
        Ok(output)
    }
}

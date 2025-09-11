use std::path::PathBuf;

use super::base_handler::BaseDriverHandler;
use super::jdbc_handler::JdbcHandler;
use super::odbc_handler::OdbcHandler;
use super::python_handler::PythonHandler;
use crate::test_discovery::Language;

pub struct DriverHandlerFactory {
    workspace_root: PathBuf,
}

impl DriverHandlerFactory {
    pub fn new(workspace_root: PathBuf) -> Self {
        Self { workspace_root }
    }

    pub fn create_handler(&self, language: &Language) -> Box<dyn BaseDriverHandler> {
        match language {
            Language::Odbc => Box::new(OdbcHandler::new(self.workspace_root.clone())),
            Language::Python => Box::new(PythonHandler::new(self.workspace_root.clone())),
            Language::Jdbc => Box::new(JdbcHandler::new(self.workspace_root.clone())),
            // Other languages don't support Breaking Changes yet
            _ => Box::new(NoOpHandler::new()),
        }
    }
}

/// No-op handler for languages that don't support Breaking Changes
struct NoOpHandler;

impl NoOpHandler {
    fn new() -> Self {
        Self
    }
}

impl BaseDriverHandler for NoOpHandler {
    fn supports_breaking_changes(&self) -> bool {
        false
    }

    fn get_breaking_changes_file_path(&self) -> PathBuf {
        PathBuf::new()
    }

    fn get_test_directory(&self) -> PathBuf {
        PathBuf::new()
    }

    fn get_test_file_extensions(&self) -> Vec<String> {
        vec![]
    }

    fn parse_breaking_changes_descriptions(
        &self,
    ) -> anyhow::Result<std::collections::HashMap<String, String>> {
        Ok(std::collections::HashMap::new())
    }

    fn extract_test_methods(&self, _content: &str) -> Vec<super::base_handler::TestMethod> {
        vec![]
    }

    fn extract_helper_method_calls(&self, _content: &str, _test_method: &str) -> Vec<String> {
        vec![]
    }

    fn find_breaking_changes_in_method(
        &self,
        _content: &str,
        _method_name: &str,
        _file_path: &std::path::Path,
    ) -> anyhow::Result<
        std::collections::HashMap<String, super::base_handler::BreakingChangeLocation>,
    > {
        Ok(std::collections::HashMap::new())
    }
}

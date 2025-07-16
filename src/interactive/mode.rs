use std::fmt::Write;
use std::io::{self, stdout};

use anyhow::Result;
use crossterm::{
    ExecutableCommand,
    event::{self, Event, KeyCode, KeyEventKind},
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{
    Terminal,
    backend::CrosstermBackend,
    widgets::{ListState, TableState},
};

use crate::interactive::helpers::{GitAction, View};
use crate::{cli::Args, gitinfo::repoinfo::RepoInfo};

/// Interactive mode for selecting and interacting with repositories
pub struct InteractiveMode {
    repos: Vec<RepoInfo>,
    table_state: TableState,
    action_list_state: ListState,
    current_view: View,
    terminal: Terminal<CrosstermBackend<io::Stdout>>,
    args: Args,
}

impl InteractiveMode {
    /// Create a new interactive mode instance
    pub fn new(repos: &[RepoInfo], args: Args) -> Result<Self> {
        let mut stdout = stdout();
        stdout.execute(EnterAlternateScreen)?;
        enable_raw_mode()?;

        let backend = CrosstermBackend::new(stdout);
        let terminal = Terminal::new(backend)?;

        let mut table_state = TableState::default();
        if !repos.is_empty() {
            table_state.select(Some(0));
        }

        let mut action_list_state = ListState::default();
        action_list_state.select(Some(0)); // Default to first action

        let mut sorted_repos = repos.to_vec();
        sorted_repos.sort_by_key(|r| r.name.to_ascii_lowercase());

        // Filter repos based on CLI options
        let filtered_repos: Vec<RepoInfo> = if args.non_clean {
            sorted_repos
                .into_iter()
                .filter(|r| r.status != crate::gitinfo::status::Status::Clean)
                .collect()
        } else {
            sorted_repos
        };

        Ok(Self {
            repos: filtered_repos,
            table_state,
            action_list_state,
            current_view: View::RepositoryList,
            terminal,
            args,
        })
    }

    /// Run the interactive mode
    pub fn run(&mut self) -> Result<()> {
        if self.repos.is_empty() {
            return Ok(());
        }

        let result = self.interactive_loop();
        self.cleanup()?;
        result
    }

    fn cleanup(&mut self) -> Result<()> {
        disable_raw_mode()?;
        self.terminal.backend_mut().execute(LeaveAlternateScreen)?;
        self.terminal.show_cursor()?;
        Ok(())
    }

    fn interactive_loop(&mut self) -> Result<()> {
        loop {
            // Clone data needed for rendering to avoid borrowing issues
            let current_view = &self.current_view;
            let args = &self.args;
            let repos = &self.repos;

            let table_state = &mut self.table_state;
            let action_list_state = &mut self.action_list_state;
            self.terminal.draw(|f| match &current_view {
                View::RepositoryList => {
                    super::draw_repository_list_ui(f, repos, table_state, args);
                }
                View::RepositoryActions(repo_index, _) => {
                    super::draw_repository_actions_ui(f, repos, *repo_index, action_list_state);
                }
                View::CommandRunning(repo_index, command_name) => {
                    super::draw_command_running_ui(f, repos, *repo_index, command_name);
                }
                View::CommandOutput(repo_index, command_name, output) => {
                    super::draw_command_output_ui(f, repos, *repo_index, command_name, output);
                }
            })?;

            if let Event::Key(key_event) = event::read()? {
                if key_event.kind == KeyEventKind::Press {
                    match self.current_view.clone() {
                        View::RepositoryList => {
                            if self.handle_repository_list_input(key_event.code) {
                                break;
                            }
                        }
                        View::RepositoryActions(repo_index, _) => {
                            if self.handle_repository_actions_input(key_event.code, repo_index)? {
                                break;
                            }
                        }
                        View::CommandRunning(_, _) => {
                            if Self::handle_command_running_input(key_event.code) {
                                break;
                            }
                        }
                        View::CommandOutput(_, _, _) => {
                            if self.handle_command_output_input(key_event.code) {
                                break;
                            }
                        }
                    }
                }
            }
        }
        Ok(())
    }

    fn handle_repository_list_input(&mut self, key_code: KeyCode) -> bool {
        match key_code {
            KeyCode::Up => {
                if let Some(selected) = self.table_state.selected() {
                    if selected > 0 {
                        self.table_state.select(Some(selected - 1));
                    }
                }
            }
            KeyCode::Down => {
                if let Some(selected) = self.table_state.selected() {
                    if selected < self.repos.len() - 1 {
                        self.table_state.select(Some(selected + 1));
                    }
                } else if !self.repos.is_empty() {
                    self.table_state.select(Some(0));
                }
            }
            KeyCode::Enter => {
                if let Some(selected) = self.table_state.selected() {
                    self.current_view = View::RepositoryActions(selected, 0);
                    self.action_list_state.select(Some(0)); // Reset to first action
                }
            }
            KeyCode::Char('q') | KeyCode::Esc => {
                return true;
            }
            KeyCode::Backspace
            | KeyCode::Left
            | KeyCode::Right
            | KeyCode::Home
            | KeyCode::End
            | KeyCode::PageUp
            | KeyCode::PageDown
            | KeyCode::Tab
            | KeyCode::BackTab
            | KeyCode::Delete
            | KeyCode::Insert
            | KeyCode::F(_)
            | KeyCode::Char(_)
            | KeyCode::Null
            | KeyCode::CapsLock
            | KeyCode::ScrollLock
            | KeyCode::NumLock
            | KeyCode::PrintScreen
            | KeyCode::Pause
            | KeyCode::Menu
            | KeyCode::KeypadBegin
            | KeyCode::Media(_)
            | KeyCode::Modifier(_) => {
                // Ignore other keys
            }
        }
        false
    }

    fn handle_repository_actions_input(
        &mut self,
        key_code: KeyCode,
        repo_index: usize,
    ) -> Result<bool> {
        let actions = GitAction::all();

        match key_code {
            KeyCode::Up => {
                if let Some(selected) = self.action_list_state.selected() {
                    if selected > 0 {
                        self.action_list_state.select(Some(selected - 1));
                        self.current_view = View::RepositoryActions(repo_index, selected - 1);
                    }
                }
            }
            KeyCode::Down => {
                if let Some(selected) = self.action_list_state.selected() {
                    if selected < actions.len() - 1 {
                        self.action_list_state.select(Some(selected + 1));
                        self.current_view = View::RepositoryActions(repo_index, selected + 1);
                    }
                } else if !actions.is_empty() {
                    self.action_list_state.select(Some(0));
                    self.current_view = View::RepositoryActions(repo_index, 0);
                }
            }
            KeyCode::Enter => {
                if let Some(selected_action_index) = self.action_list_state.selected() {
                    if let Some(action) = actions.get(selected_action_index) {
                        match action {
                            GitAction::Status => {
                                // Show loading state first
                                self.current_view =
                                    View::CommandRunning(repo_index, "Git Status".to_owned());
                                self.force_redraw()?;

                                let output = Self::execute_git_status(&self.repos[repo_index])?;
                                self.current_view = View::CommandOutput(
                                    repo_index,
                                    "Git Status".to_owned(),
                                    output,
                                );
                            }
                            GitAction::Push => {
                                // Show loading state first
                                self.current_view =
                                    View::CommandRunning(repo_index, "Git Push".to_owned());
                                self.force_redraw()?;

                                let output = Self::execute_git_push(&self.repos[repo_index])?;
                                self.current_view =
                                    View::CommandOutput(repo_index, "Git Push".to_owned(), output);
                            }
                            GitAction::Fetch => {
                                // Show loading state first
                                self.current_view =
                                    View::CommandRunning(repo_index, "Git Fetch".to_owned());
                                self.force_redraw()?;

                                let output = Self::execute_git_fetch(&self.repos[repo_index])?;
                                self.current_view =
                                    View::CommandOutput(repo_index, "Git Fetch".to_owned(), output);
                            }
                            GitAction::Pull => {
                                // Show loading state first
                                self.current_view =
                                    View::CommandRunning(repo_index, "Git Pull".to_owned());
                                self.force_redraw()?;

                                let output = Self::execute_git_pull(&self.repos[repo_index])?;
                                self.current_view =
                                    View::CommandOutput(repo_index, "Git Pull".to_owned(), output);
                            }
                            GitAction::Back => {
                                self.current_view = View::RepositoryList;
                            }
                        }
                    }
                }
            }
            KeyCode::Esc | KeyCode::Backspace => {
                self.current_view = View::RepositoryList;
            }
            KeyCode::Char('q') => {
                return Ok(true);
            }
            KeyCode::Left
            | KeyCode::Right
            | KeyCode::Home
            | KeyCode::End
            | KeyCode::PageUp
            | KeyCode::PageDown
            | KeyCode::Tab
            | KeyCode::BackTab
            | KeyCode::Delete
            | KeyCode::Insert
            | KeyCode::F(_)
            | KeyCode::Char(_)
            | KeyCode::Null
            | KeyCode::CapsLock
            | KeyCode::ScrollLock
            | KeyCode::NumLock
            | KeyCode::PrintScreen
            | KeyCode::Pause
            | KeyCode::Menu
            | KeyCode::KeypadBegin
            | KeyCode::Media(_)
            | KeyCode::Modifier(_) => {
                // Ignore other keys
            }
        }
        Ok(false)
    }

    fn handle_command_running_input(key_code: KeyCode) -> bool {
        key_code == KeyCode::Char('q')
    }

    fn force_redraw(&mut self) -> Result<()> {
        let current_view = self.current_view.clone();
        let repos = self.repos.clone();
        let args = &self.args;

        let table_state = &mut self.table_state;
        let action_list_state = &mut self.action_list_state;
        self.terminal.draw(|f| match &current_view {
            View::RepositoryList => {
                super::draw_repository_list_ui(f, &repos, table_state, args);
            }
            View::RepositoryActions(repo_index, _) => {
                super::draw_repository_actions_ui(f, &repos, *repo_index, action_list_state);
            }
            View::CommandRunning(repo_index, command_name) => {
                super::draw_command_running_ui(f, &repos, *repo_index, command_name);
            }
            View::CommandOutput(repo_index, command_name, output) => {
                super::draw_command_output_ui(f, &repos, *repo_index, command_name, output);
            }
        })?;
        Ok(())
    }

    fn handle_command_output_input(&mut self, key_code: KeyCode) -> bool {
        match key_code {
            KeyCode::Esc | KeyCode::Backspace | KeyCode::Enter => {
                // Go back to the repository actions view
                if let View::CommandOutput(repo_index, _, _) = &self.current_view {
                    let repo_index = *repo_index;
                    self.current_view = View::RepositoryActions(
                        repo_index,
                        self.action_list_state.selected().unwrap_or(0),
                    );
                }
            }
            KeyCode::Char('q') => {
                return true;
            }
            KeyCode::Left
            | KeyCode::Right
            | KeyCode::Up
            | KeyCode::Down
            | KeyCode::Home
            | KeyCode::End
            | KeyCode::PageUp
            | KeyCode::PageDown
            | KeyCode::Tab
            | KeyCode::BackTab
            | KeyCode::Delete
            | KeyCode::Insert
            | KeyCode::F(_)
            | KeyCode::Char(_)
            | KeyCode::Null
            | KeyCode::CapsLock
            | KeyCode::ScrollLock
            | KeyCode::NumLock
            | KeyCode::PrintScreen
            | KeyCode::Pause
            | KeyCode::Menu
            | KeyCode::KeypadBegin
            | KeyCode::Media(_)
            | KeyCode::Modifier(_) => {
                // Ignore other keys
            }
        }
        false
    }

    fn execute_git_status(repo: &RepoInfo) -> Result<String> {
        let output = std::process::Command::new("git")
            .arg("status")
            .current_dir(&repo.path)
            .output()?;

        let mut result = format!("📋 Git Status for {}\n", repo.name);
        write!(result, "📍 Path: {}\n\n", repo.path.display()).unwrap();

        if output.status.success() {
            result.push_str(&String::from_utf8_lossy(&output.stdout));
        } else {
            result.push_str("❌ Error running git status:\n");
            result.push_str(&String::from_utf8_lossy(&output.stderr));
        }

        Ok(result)
    }

    fn execute_git_push(repo: &RepoInfo) -> Result<String> {
        let output = std::process::Command::new("git")
            .arg("push")
            .current_dir(&repo.path)
            .output()?;

        let mut result = format!("📤 Git Push for {}\n", repo.name);
        write!(result, "📍 Path: {}\n\n", repo.path.display()).unwrap();

        if output.status.success() {
            result.push_str("✅ Push completed successfully!\n\n");
            result.push_str(&String::from_utf8_lossy(&output.stdout));
            if !output.stderr.is_empty() {
                result.push_str("\n📄 Additional info:\n");
                result.push_str(&String::from_utf8_lossy(&output.stderr));
            }
        } else {
            result.push_str("❌ Error during git push:\n");
            result.push_str(&String::from_utf8_lossy(&output.stderr));
        }

        Ok(result)
    }

    fn execute_git_fetch(repo: &RepoInfo) -> Result<String> {
        let output = std::process::Command::new("git")
            .arg("fetch")
            .current_dir(&repo.path)
            .output()?;

        let mut result = format!("📥 Git Fetch for {}\n", repo.name);
        write!(result, "📍 Path: {}\n\n", repo.path.display()).unwrap();

        if output.status.success() {
            result.push_str("✅ Fetch completed successfully!\n\n");
            if !output.stdout.is_empty() {
                result.push_str(&String::from_utf8_lossy(&output.stdout));
            }
            if !output.stderr.is_empty() {
                result.push_str("\n📄 Additional info:\n");
                result.push_str(&String::from_utf8_lossy(&output.stderr));
            }
            if output.stdout.is_empty() && output.stderr.is_empty() {
                result.push_str("📡 Already up to date with remote.");
            }
        } else {
            result.push_str("❌ Error during git fetch:\n");
            result.push_str(&String::from_utf8_lossy(&output.stderr));
        }

        Ok(result)
    }

    fn execute_git_pull(repo: &RepoInfo) -> Result<String> {
        let output = std::process::Command::new("git")
            .arg("pull")
            .current_dir(&repo.path)
            .output()?;

        let mut result = format!("⬇️ Git Pull for {}\n", repo.name);
        write!(result, "📍 Path: {}\n\n", repo.path.display()).unwrap();

        if output.status.success() {
            result.push_str("✅ Pull completed successfully!\n\n");
            result.push_str(&String::from_utf8_lossy(&output.stdout));
            if !output.stderr.is_empty() {
                result.push_str("\n📄 Additional info:\n");
                result.push_str(&String::from_utf8_lossy(&output.stderr));
            }
        } else {
            result.push_str("❌ Error during git pull:\n");
            result.push_str(&String::from_utf8_lossy(&output.stderr));
        }

        Ok(result)
    }
}

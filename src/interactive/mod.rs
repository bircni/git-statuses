use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{
        Block, Borders, Cell, List, ListItem, ListState, Paragraph, Row, Table, TableState, Wrap,
    },
};

use crate::interactive::helpers::GitAction;
use crate::{cli::Args, gitinfo::repoinfo::RepoInfo};

mod helpers;
pub mod mode;

#[expect(
    clippy::too_many_lines,
    reason = "This function handles the entire interactive UI drawing logic, which can be complex."
)]
fn draw_repository_list_ui(
    f: &mut ratatui::Frame<'_>,
    repos: &[RepoInfo],
    table_state: &mut TableState,
    args: &Args,
) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Title
            Constraint::Min(1),    // Table
            Constraint::Length(3), // Help
        ])
        .split(f.area());

    // Title
    let title = Paragraph::new("🔧 Interactive Mode - Repository Selection")
        .style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )
        .block(Block::default().borders(Borders::ALL));
    f.render_widget(title, chunks[0]);

    // Repository table
    let mut headers = vec!["Directory", "Branch", "Local", "Commits", "Status"];
    if args.remote {
        headers.push("Remote");
    }
    if args.path {
        headers.push("Path");
    }

    let header_cells = headers
        .iter()
        .map(|h| Cell::from(*h).style(Style::default().add_modifier(Modifier::BOLD)))
        .collect::<Vec<_>>();
    let header = Row::new(header_cells);

    let rows = repos.iter().enumerate().map(|(i, repo)| {
        let repo_color = repo.status.ratatui_color();

        let name_style = if Some(i) == table_state.selected() {
            Style::default()
                .fg(repo_color)
                .bg(Color::Blue)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(repo_color)
        };

        let status_style = if Some(i) == table_state.selected() {
            Style::default()
                .fg(repo_color)
                .bg(Color::Blue)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(repo_color)
        };

        let mut cells = vec![
            Cell::from(repo.name.clone()).style(name_style),
            Cell::from(repo.branch.clone()),
            Cell::from(repo.format_local_status()),
            Cell::from(repo.commits.to_string()),
            Cell::from(repo.format_status_with_stash()).style(status_style),
        ];

        if args.remote {
            cells.push(Cell::from(repo.remote_url.as_deref().unwrap_or("-")));
        }
        if args.path {
            cells.push(Cell::from(repo.path.display().to_string()));
        }

        Row::new(cells)
    });

    let widths = if args.remote && args.path {
        vec![
            Constraint::Percentage(15), // Directory
            Constraint::Percentage(15), // Branch
            Constraint::Percentage(10), // Local
            Constraint::Percentage(10), // Commits
            Constraint::Percentage(15), // Status
            Constraint::Percentage(20), // Remote
            Constraint::Percentage(15), // Path
        ]
    } else if args.path || args.remote {
        vec![
            Constraint::Percentage(20), // Directory
            Constraint::Percentage(15), // Branch
            Constraint::Percentage(15), // Local
            Constraint::Percentage(10), // Commits
            Constraint::Percentage(15), // Status
            Constraint::Percentage(25), // Path
        ]
    } else {
        vec![
            Constraint::Percentage(25), // Directory
            Constraint::Percentage(20), // Branch
            Constraint::Percentage(20), // Local
            Constraint::Percentage(15), // Commits
            Constraint::Percentage(20), // Status
        ]
    };

    let table = Table::new(rows, widths)
        .header(header)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("📂 Repositories"),
        )
        .row_highlight_style(
            Style::default()
                .bg(Color::Blue)
                .add_modifier(Modifier::BOLD),
        );

    f.render_stateful_widget(table, chunks[1], table_state);

    // Help text
    let help_text =
        Paragraph::new("💡 Navigation: ↑/↓ arrows to select, Enter to interact, 'q' to quit")
            .style(Style::default().fg(Color::Gray))
            .block(Block::default().borders(Borders::ALL))
            .wrap(Wrap { trim: true });
    f.render_widget(help_text, chunks[2]);
}

fn draw_repository_actions_ui(
    f: &mut ratatui::Frame<'_>,
    repos: &[RepoInfo],
    repo_index: usize,
    action_list_state: &mut ListState,
) {
    if let Some(repo) = repos.get(repo_index) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(8), // Repository info
                Constraint::Min(1),    // Actions
                Constraint::Length(3), // Help
            ])
            .split(f.area());

        // Repository info
        let mut info_lines = vec![
            Line::from(vec![
                Span::styled("🔧 Repository: ", Style::default().fg(Color::Cyan)),
                Span::from(repo.name.clone()),
            ]),
            Line::from(vec![
                Span::styled("📍 Path: ", Style::default().fg(Color::Yellow)),
                Span::from(repo.path.display().to_string()),
            ]),
            Line::from(vec![
                Span::styled("🌿 Branch: ", Style::default().fg(Color::Green)),
                Span::from(repo.branch.clone()),
            ]),
            Line::from(vec![
                Span::styled("📊 Status: ", Style::default().fg(Color::Magenta)),
                Span::from(repo.status.to_string()),
            ]),
            Line::from(vec![
                Span::styled("🔄 Local: ", Style::default().fg(Color::Blue)),
                Span::from(repo.format_local_status()),
            ]),
        ];

        if let Some(ref remote_url) = repo.remote_url {
            info_lines.push(Line::from(vec![
                Span::styled("🌐 Remote: ", Style::default().fg(Color::Cyan)),
                Span::from(remote_url.clone()),
            ]));
        }

        let repo_info = Paragraph::new(Text::from(info_lines))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Repository Details"),
            )
            .wrap(Wrap { trim: true });
        f.render_widget(repo_info, chunks[0]);

        // Actions list
        let actions = GitAction::all();
        let action_items: Vec<ListItem<'_>> = actions
            .iter()
            .map(|action| ListItem::new(action.as_str()))
            .collect();

        let actions_list = List::new(action_items)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("🛠️ Available Actions"),
            )
            .highlight_style(
                Style::default()
                    .bg(Color::Blue)
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol("▶ ");

        f.render_stateful_widget(actions_list, chunks[1], action_list_state);

        // Help text
        let help_text = Paragraph::new("💡 Navigation: ↑/↓ arrows to select, Enter to execute, Esc/Backspace to go back, 'q' to quit")
            .style(Style::default().fg(Color::Gray))
            .block(Block::default().borders(Borders::ALL))
            .wrap(Wrap { trim: true });
        f.render_widget(help_text, chunks[2]);
    }
}

fn draw_command_running_ui(
    f: &mut ratatui::Frame<'_>,
    repos: &[RepoInfo],
    repo_index: usize,
    command_name: &str,
) {
    if let Some(repo) = repos.get(repo_index) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(5), // Repository and command info
                Constraint::Min(1),    // Loading indicator
                Constraint::Length(3), // Help
            ])
            .split(f.area());

        // Repository and command info
        let info_lines = vec![
            Line::from(vec![
                Span::styled("🔧 Repository: ", Style::default().fg(Color::Cyan)),
                Span::from(repo.name.clone()),
            ]),
            Line::from(vec![
                Span::styled("⚡ Command: ", Style::default().fg(Color::Yellow)),
                Span::from(command_name),
            ]),
        ];

        let info = Paragraph::new(Text::from(info_lines))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Command Information"),
            )
            .wrap(Wrap { trim: true });
        f.render_widget(info, chunks[0]);

        // Loading indicator
        let loading_text = vec![
            Line::from(""),
            Line::from(vec![
                Span::styled("⏳ ", Style::default().fg(Color::Yellow)),
                Span::styled(
                    "Executing command...",
                    Style::default()
                        .fg(Color::White)
                        .add_modifier(Modifier::BOLD),
                ),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled("🔄 ", Style::default().fg(Color::Blue)),
                Span::styled(
                    "Please wait while the git command is running.",
                    Style::default().fg(Color::Gray),
                ),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled("💡 ", Style::default().fg(Color::Green)),
                Span::styled(
                    "This may take a moment depending on your repository size and network connection.",
                    Style::default().fg(Color::Gray),
                ),
            ]),
        ];

        let loading_paragraph = Paragraph::new(Text::from(loading_text))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("⏳ Running Command"),
            )
            .wrap(Wrap { trim: true });
        f.render_widget(loading_paragraph, chunks[1]);

        // Help text
        let help_text =
            Paragraph::new("💡 Press 'q' to quit (this will not cancel the running command)")
                .style(Style::default().fg(Color::Gray))
                .block(Block::default().borders(Borders::ALL))
                .wrap(Wrap { trim: true });
        f.render_widget(help_text, chunks[2]);
    }
}

fn draw_command_output_ui(
    f: &mut ratatui::Frame<'_>,
    repos: &[RepoInfo],
    repo_index: usize,
    command_name: &str,
    output: &str,
) {
    if let Some(repo) = repos.get(repo_index) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(5), // Repository and command info
                Constraint::Min(1),    // Command output
                Constraint::Length(3), // Help
            ])
            .split(f.area());

        // Repository and command info
        let info_lines = vec![
            Line::from(vec![
                Span::styled("🔧 Repository: ", Style::default().fg(Color::Cyan)),
                Span::from(repo.name.clone()),
            ]),
            Line::from(vec![
                Span::styled("⚡ Command: ", Style::default().fg(Color::Yellow)),
                Span::from(command_name),
            ]),
        ];

        let info = Paragraph::new(Text::from(info_lines))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Command Information"),
            )
            .wrap(Wrap { trim: true });
        f.render_widget(info, chunks[0]);

        // Command output
        let output_paragraph = Paragraph::new(output)
            .block(Block::default().borders(Borders::ALL).title("📄 Output"))
            .wrap(Wrap { trim: true })
            .scroll((0, 0));
        f.render_widget(output_paragraph, chunks[1]);

        // Help text
        let help_text =
            Paragraph::new("💡 Press Enter/Esc/Backspace to go back to actions, 'q' to quit")
                .style(Style::default().fg(Color::Gray))
                .block(Block::default().borders(Borders::ALL))
                .wrap(Wrap { trim: true });
        f.render_widget(help_text, chunks[2]);
    }
}

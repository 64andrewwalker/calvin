//! Keyboard input handling and interactive loop.
//!
//! This module provides functions for mapping keyboard events to tree actions
//! and running the interactive terminal loop.

use crossterm::event::KeyEvent;

use super::menu::{TreeAction, TreeMenu};

/// Convert a keyboard event to a TreeAction
pub fn key_to_action(key: KeyEvent) -> Option<TreeAction> {
    use crossterm::event::KeyCode;

    match key.code {
        KeyCode::Up | KeyCode::Char('k') => Some(TreeAction::Up),
        KeyCode::Down | KeyCode::Char('j') => Some(TreeAction::Down),
        KeyCode::Char(' ') => Some(TreeAction::Toggle),
        KeyCode::Right | KeyCode::Char('l') => Some(TreeAction::Expand),
        KeyCode::Left | KeyCode::Char('h') | KeyCode::Backspace => Some(TreeAction::Collapse),
        KeyCode::Char('a') => Some(TreeAction::SelectAll),
        KeyCode::Char('n') => Some(TreeAction::SelectNone),
        KeyCode::Char('i') => Some(TreeAction::Invert),
        KeyCode::Char('q') | KeyCode::Esc => Some(TreeAction::Quit),
        _ => None,
    }
}

/// Run the tree menu interactively
/// Returns the selected keys if confirmed, None if quit
pub fn run_interactive(
    menu: &mut TreeMenu,
    supports_unicode: bool,
) -> std::io::Result<Option<Vec<String>>> {
    use crossterm::{
        cursor,
        event::{self, Event, KeyCode, KeyEventKind},
        execute,
        terminal::{self, ClearType},
    };
    use std::io::{stdout, Write};

    // Enable raw mode
    terminal::enable_raw_mode()?;
    let mut stdout = stdout();

    // Helper to render the full UI
    let render_ui = |stdout: &mut std::io::Stdout, menu: &TreeMenu| -> std::io::Result<()> {
        // Clear entire screen and move to top
        execute!(
            stdout,
            terminal::Clear(ClearType::All),
            cursor::MoveTo(0, 0)
        )?;

        // Header
        println!("🧹 Calvin Clean\r");
        println!("\r");

        // Render tree
        let rendered = menu.render(supports_unicode);
        for line in rendered.lines() {
            print!("{}\r\n", line);
        }

        // Separator
        println!("───────────────────────────────────────────────────────────────\r");

        // Status bar
        let status = menu.render_status_bar(supports_unicode);
        for line in status.lines() {
            print!("{}\r\n", line);
        }
        println!("\r");

        // Help bar
        let help = menu.render_help_bar();
        for line in help.lines() {
            print!("{}\r\n", line);
        }

        stdout.flush()?;
        Ok(())
    };

    // Hide cursor
    execute!(stdout, cursor::Hide)?;

    // Initial render
    render_ui(&mut stdout, menu)?;

    let result = loop {
        // Wait for key event
        if let Event::Key(key) = event::read()? {
            if key.kind != KeyEventKind::Press {
                continue;
            }

            // Enter always confirms selection
            if key.code == KeyCode::Enter {
                break Some(menu.selected_keys());
            }

            if let Some(action) = key_to_action(key) {
                match action {
                    TreeAction::Confirm => break Some(menu.selected_keys()),
                    TreeAction::Quit => break None,
                    _ => {
                        menu.handle_action(action);
                        // Redraw after action
                        render_ui(&mut stdout, menu)?;
                    }
                }
            }
        }
    };

    // Restore terminal
    execute!(
        stdout,
        cursor::Show,
        terminal::Clear(ClearType::All),
        cursor::MoveTo(0, 0)
    )?;
    terminal::disable_raw_mode()?;

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::{KeyCode, KeyModifiers};

    #[test]
    fn key_to_action_arrow_keys() {
        assert_eq!(
            key_to_action(KeyEvent::new(KeyCode::Up, KeyModifiers::NONE)),
            Some(TreeAction::Up)
        );
        assert_eq!(
            key_to_action(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE)),
            Some(TreeAction::Down)
        );
        assert_eq!(
            key_to_action(KeyEvent::new(KeyCode::Left, KeyModifiers::NONE)),
            Some(TreeAction::Collapse)
        );
        assert_eq!(
            key_to_action(KeyEvent::new(KeyCode::Right, KeyModifiers::NONE)),
            Some(TreeAction::Expand)
        );
    }

    #[test]
    fn key_to_action_vim_keys() {
        assert_eq!(
            key_to_action(KeyEvent::new(KeyCode::Char('k'), KeyModifiers::NONE)),
            Some(TreeAction::Up)
        );
        assert_eq!(
            key_to_action(KeyEvent::new(KeyCode::Char('j'), KeyModifiers::NONE)),
            Some(TreeAction::Down)
        );
        assert_eq!(
            key_to_action(KeyEvent::new(KeyCode::Char('h'), KeyModifiers::NONE)),
            Some(TreeAction::Collapse)
        );
        assert_eq!(
            key_to_action(KeyEvent::new(KeyCode::Char('l'), KeyModifiers::NONE)),
            Some(TreeAction::Expand)
        );
    }

    #[test]
    fn key_to_action_bulk_shortcuts() {
        assert_eq!(
            key_to_action(KeyEvent::new(KeyCode::Char('a'), KeyModifiers::NONE)),
            Some(TreeAction::SelectAll)
        );
        assert_eq!(
            key_to_action(KeyEvent::new(KeyCode::Char('n'), KeyModifiers::NONE)),
            Some(TreeAction::SelectNone)
        );
        assert_eq!(
            key_to_action(KeyEvent::new(KeyCode::Char('i'), KeyModifiers::NONE)),
            Some(TreeAction::Invert)
        );
    }

    #[test]
    fn key_to_action_quit_keys() {
        assert_eq!(
            key_to_action(KeyEvent::new(KeyCode::Char('q'), KeyModifiers::NONE)),
            Some(TreeAction::Quit)
        );
        assert_eq!(
            key_to_action(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE)),
            Some(TreeAction::Quit)
        );
    }

    #[test]
    fn key_to_action_unknown_key() {
        assert_eq!(
            key_to_action(KeyEvent::new(KeyCode::Char('z'), KeyModifiers::NONE)),
            None
        );
        assert_eq!(
            key_to_action(KeyEvent::new(KeyCode::F(1), KeyModifiers::NONE)),
            None
        );
    }

    #[test]
    fn key_to_action_space_toggle() {
        assert_eq!(
            key_to_action(KeyEvent::new(KeyCode::Char(' '), KeyModifiers::NONE)),
            Some(TreeAction::Toggle)
        );
    }
}

use std::io::{self, Write};
use std::process::{Command, Stdio};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MenuOutcome {
  Selected(Vec<usize>),
  Cancelled,
  Back,
  Unavailable,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TextPromptOutcome {
  Text(String),
  Cancelled,
  Unavailable,
}

pub fn prompt_text(prompt: &str, backend: Option<&str>) -> io::Result<TextPromptOutcome> {
  let Some(program) = backend else {
    return Ok(TextPromptOutcome::Unavailable);
  };

  let mut command = Command::new(program);
  match program.rsplit('/').next().unwrap_or(program) {
    // The shell implementation reads the search string from the terminal
    // when fzf is the active menu backend.
    "fzf" => return Ok(TextPromptOutcome::Unavailable),
    "rofi" => {
      command.args(["-dmenu", "-i", "-p", prompt]);
    }
    "dmenu" => {
      command.args(["-l", "20", "-p", prompt]);
    }
    _ => {
      command.arg(prompt);
    }
  }

  let extra_flags: Vec<String> = std::env::var("ANI_CLI_MENU_FLAGS")
    .unwrap_or_default()
    .split_whitespace()
    .map(str::to_owned)
    .collect();
  command.args(extra_flags);
  command
    .stdin(Stdio::piped())
    .stdout(Stdio::piped())
    .stderr(Stdio::inherit());

  let mut child = match command.spawn() {
    Ok(child) => child,
    Err(error) if error.kind() == io::ErrorKind::NotFound => {
      return Ok(TextPromptOutcome::Unavailable);
    }
    Err(error) => return Err(error),
  };
  drop(child.stdin.take());
  let output = child.wait_with_output()?;

  if !output.status.success() {
    if matches!(output.status.code(), Some(1 | 130)) {
      return Ok(TextPromptOutcome::Cancelled);
    }
    return Err(io::Error::other(format!(
      "{program} exited with status {}",
      output.status
    )));
  }

  Ok(
    parse_prompt_value(&String::from_utf8_lossy(&output.stdout))
      .map(TextPromptOutcome::Text)
      .unwrap_or(TextPromptOutcome::Cancelled),
  )
}

fn parse_prompt_value(output: &str) -> Option<String> {
  let value = output.trim();
  (!value.is_empty()).then(|| value.to_owned())
}

/// Run an external menu using one-based indices prefixed to each item.
/// If no backend was explicitly configured and fzf is missing, callers can use
/// their built-in numbered prompt instead.
pub fn select(
  items: &[String],
  prompt: &str,
  backend: Option<&str>,
  multi: bool,
) -> io::Result<MenuOutcome> {
  if items.is_empty() {
    return Err(io::Error::new(
      io::ErrorKind::InvalidInput,
      "cannot show a menu with no entries",
    ));
  }

  let explicitly_configured = backend.is_some();
  let program = backend.unwrap_or("fzf");
  let mut command = Command::new(program);
  match program.rsplit('/').next().unwrap_or(program) {
    "fzf" => {
      command.args([
        "--reverse",
        "--cycle",
        "--prompt",
        prompt,
        "--expect=backspace",
      ]);
      if multi {
        command.arg("--multi");
      }
    }
    "rofi" => {
      command.args(["-sort", "-dmenu", "-i", "-p", prompt]);
      if multi {
        command.arg("-multi-select");
      }
    }
    "dmenu" => {
      command.args(["-l", "20", "-p", prompt]);
    }
    _ => {
      command.arg(prompt);
    }
  }
  let extra_flags: Vec<String> = std::env::var("ANI_CLI_MENU_FLAGS")
    .unwrap_or_default()
    .split_whitespace()
    .map(str::to_owned)
    .collect();
  command.args(extra_flags);
  command
    .stdin(Stdio::piped())
    .stdout(Stdio::piped())
    .stderr(Stdio::inherit());

  let mut child = match command.spawn() {
    Ok(child) => child,
    Err(error) if error.kind() == io::ErrorKind::NotFound && !explicitly_configured => {
      return Ok(MenuOutcome::Unavailable);
    }
    Err(error) => return Err(error),
  };

  {
    let stdin = child
      .stdin
      .as_mut()
      .ok_or_else(|| io::Error::other("menu process did not expose standard input"))?;
    for (index, item) in items.iter().enumerate() {
      writeln!(stdin, "{}\t{}", index + 1, item.replace('\t', " "))?;
    }
  }
  drop(child.stdin.take());

  let output = child.wait_with_output()?;
  if !output.status.success() {
    // fzf uses 130 for Escape; rofi/dmenu commonly use 1.
    if matches!(output.status.code(), Some(1 | 130)) {
      return Ok(MenuOutcome::Cancelled);
    }
    return Err(io::Error::other(format!(
      "{program} exited with status {}",
      output.status
    )));
  }

  let stdout = String::from_utf8_lossy(&output.stdout);
  if stdout.lines().next() == Some("backspace") {
    return Ok(MenuOutcome::Back);
  }

  let mut selected = Vec::new();
  for line in stdout.lines() {
    if let Some(index) = parse_selected_index(line, items) {
      if !selected.contains(&index) {
        selected.push(index);
      }
    }
  }
  if !multi {
    selected.truncate(1);
  }
  if selected.is_empty() {
    Ok(MenuOutcome::Cancelled)
  } else {
    Ok(MenuOutcome::Selected(selected))
  }
}

fn parse_selected_index(line: &str, items: &[String]) -> Option<usize> {
  if let Some((number, _)) = line.split_once('\t') {
    if let Ok(number) = number.trim().parse::<usize>() {
      if (1..=items.len()).contains(&number) {
        return Some(number - 1);
      }
    }
  }
  items.iter().position(|item| item == line.trim())
}

#[cfg(test)]
#[path = "../../tests/unit/cli/menu.rs"]
mod tests;

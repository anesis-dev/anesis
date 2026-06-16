use std::time::Duration;

use indicatif::{ProgressBar, ProgressStyle};

pub fn spinner(msg: impl Into<String>) -> ProgressBar {
  let pb = ProgressBar::new_spinner();
  pb.set_style(
    ProgressStyle::with_template("{spinner:.cyan} {msg}")
      .unwrap()
      .tick_strings(&["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"]),
  );
  pb.set_message(msg.into());
  pb.enable_steady_tick(Duration::from_millis(80));
  pb
}

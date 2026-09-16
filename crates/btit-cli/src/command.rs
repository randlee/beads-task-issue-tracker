//! [`new_command`]: the platform-adjusted `std::process::Command` every spawn starts from.

use std::process::Command;

/// Creates a Command with platform-specific flags.
/// On Windows, sets `CREATE_NO_WINDOW` to prevent console popups.
#[must_use]
#[cfg_attr(
    target_os = "windows",
    expect(
        clippy::unreadable_literal,
        reason = "CREATE_NO_WINDOW literal moved verbatim from btit-app cli.rs in b-4"
    )
)]
pub fn new_command(program: &str) -> Command {
    #[cfg_attr(
        not(target_os = "windows"),
        expect(
            unused_mut,
            reason = "`mut` is needed only by the Windows creation_flags call below"
        )
    )]
    let mut cmd = Command::new(program);
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW
    }
    cmd
}

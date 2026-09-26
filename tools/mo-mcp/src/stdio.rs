//! Process transport only; the computational core has no standard I/O owner.
//! A pending Unix pipe read/write is reactor-owned and can be dropped without
//! leaving an uncancellable blocking worker behind during runtime shutdown.
#[cfg(unix)]
pub const BLOCKING_THREADS: usize = 0;

#[cfg(unix)]
pub fn open() -> std::io::Result<(
    tokio::net::unix::pipe::Receiver,
    tokio::net::unix::pipe::Sender,
)> {
    use std::os::fd::AsFd;
    use tokio::net::unix::pipe::{Receiver, Sender};
    // Checked constructors validate FIFO/access mode, enable nonblocking I/O,
    // and register readiness. Neither standard handle is closed or borrowed
    // by a blocking task. MCP subprocess hosts must supply pipes on both ends.
    let input = std::io::stdin().as_fd().try_clone_to_owned()?;
    let output = std::io::stdout().as_fd().try_clone_to_owned()?;
    Ok((
        Receiver::from_owned_fd(input)?,
        Sender::from_owned_fd(output)?,
    ))
}

// Preserve the existing non-Unix development implementation. Its blocking
// standard I/O cannot claim the Unix shutdown guarantee; a cancellable Windows
// transport and native lifecycle validation remain required before release.
#[cfg(not(unix))]
pub const BLOCKING_THREADS: usize = 2;

#[cfg(not(unix))]
pub fn open() -> std::io::Result<(tokio::io::Stdin, tokio::io::Stdout)> {
    Ok((tokio::io::stdin(), tokio::io::stdout()))
}

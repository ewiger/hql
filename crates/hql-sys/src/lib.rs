//! The operating-system calls of `hql` that need `unsafe`, each behind a
//! safe function. `hql` itself forbids `unsafe` (its `src/lib.rs`); every
//! call that cannot be made without it lives here and nowhere else
//! (HQL-0064, the companion note on child processes, the section
//! "Windows").

/// A signal `hql` sends to a process group it started.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Stop {
    /// Asks the processes to end: `SIGTERM`.
    Term,
    /// Ends the processes: `SIGKILL`.
    Kill,
}

/// Sends `stop` to every process of the process group `group`.
///
/// `group` is the identifier of a group's leader, a child started with
/// `std::os::unix::process::CommandExt::process_group(0)`.
///
/// # Errors
///
/// The group no longer exists, or this process may not signal it.
#[cfg(unix)]
pub fn stop_group(group: u32, stop: Stop) -> std::io::Result<()> {
    let group = libc::pid_t::try_from(group)
        .map_err(|_| std::io::Error::from(std::io::ErrorKind::InvalidInput))?;
    let signal = match stop {
        Stop::Term => libc::SIGTERM,
        Stop::Kill => libc::SIGKILL,
    };
    // SAFETY: `killpg` takes two integers and touches no memory of this
    // process; a group that does not exist is an error it returns.
    if unsafe { libc::killpg(group, signal) } == 0 {
        Ok(())
    } else {
        Err(std::io::Error::last_os_error())
    }
}

#[cfg(all(test, unix))]
mod tests {
    use super::{Stop, stop_group};
    use std::os::unix::process::CommandExt;
    use std::process::Command;

    #[test]
    fn a_stopped_group_ends_its_leader_and_its_children() {
        let mut leader = Command::new("sh")
            .args(["-c", "sleep 30 & sleep 30"])
            .process_group(0)
            .spawn()
            .unwrap();
        stop_group(leader.id(), Stop::Kill).unwrap();
        let status = leader.wait().unwrap();
        assert!(!status.success());
        // The background `sleep` was killed too: once init has reaped it, no
        // process of the group is left to signal.
        let gone = (0..100).any(|_| {
            std::thread::sleep(std::time::Duration::from_millis(10));
            stop_group(leader.id(), Stop::Term).is_err()
        });
        assert!(gone, "a process of the group survived");
    }
}

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

/// The signals that end a process by default and that a person or a
/// supervisor sends to stop it: `SIGINT`, `SIGTERM` and `SIGHUP`.
#[cfg(unix)]
const ENDING: [libc::c_int; 3] = [libc::SIGINT, libc::SIGTERM, libc::SIGHUP];

/// The end of the pipe the handler writes each caught signal to.
#[cfg(unix)]
static WRITTEN_TO: std::sync::atomic::AtomicI32 = std::sync::atomic::AtomicI32::new(-1);

/// Where this thread's `errno` is, so the handler leaves it as it found it.
#[cfg(unix)]
fn errno() -> Option<*mut libc::c_int> {
    #[cfg(any(target_os = "macos", target_os = "ios", target_os = "freebsd"))]
    // SAFETY: `__error` gives the calling thread's `errno`.
    return Some(unsafe { libc::__error() });
    #[cfg(any(target_os = "linux", target_os = "android"))]
    // SAFETY: `__errno_location` gives the calling thread's `errno`.
    return Some(unsafe { libc::__errno_location() });
    #[allow(unreachable_code)]
    None
}

/// Writes the caught signal's number to the pipe: `write` is one of the
/// calls a handler may make, and the pipe never blocks it.
#[cfg(unix)]
extern "C" fn caught(signal: libc::c_int) {
    let place = errno();
    // SAFETY: `place` is this thread's `errno`.
    let saved = place.map(|place| unsafe { *place });
    let descriptor = WRITTEN_TO.load(std::sync::atomic::Ordering::Relaxed);
    if descriptor >= 0 {
        let byte = u8::try_from(signal).unwrap_or(u8::MAX);
        // SAFETY: one byte is written from a valid place to an open pipe.
        unsafe {
            libc::write(descriptor, (&raw const byte).cast(), 1);
        }
    }
    if let (Some(place), Some(saved)) = (place, saved) {
        // SAFETY: as above.
        unsafe { *place = saved };
    }
}

/// The signals `SIGINT`, `SIGTERM` and `SIGHUP`, caught: they no longer end
/// the process, and [`EndingSignals::wait`] gives each as it arrives.
///
/// A caught signal returns to its default action in a child when the child
/// runs its program, so a child is ended by them as any process is.
#[cfg(unix)]
#[derive(Debug)]
pub struct EndingSignals {
    read: std::fs::File,
}

#[cfg(unix)]
fn checked(code: libc::c_int) -> std::io::Result<libc::c_int> {
    if code == -1 {
        Err(std::io::Error::last_os_error())
    } else {
        Ok(code)
    }
}

#[cfg(unix)]
impl EndingSignals {
    /// Catches the three signals. Once per process.
    ///
    /// # Errors
    ///
    /// They are already caught, or the pipe or a handler could not be made.
    pub fn catch() -> std::io::Result<Self> {
        use std::os::fd::FromRawFd;
        let mut ends = [0 as libc::c_int; 2];
        // SAFETY: `ends` holds the two descriptors `pipe` writes.
        checked(unsafe { libc::pipe(ends.as_mut_ptr()) })?;
        let [read, write] = ends;
        for descriptor in ends {
            // SAFETY: `descriptor` was just opened by `pipe`.
            unsafe {
                checked(libc::fcntl(descriptor, libc::F_SETFD, libc::FD_CLOEXEC))?;
            }
        }
        // SAFETY: as above.
        unsafe {
            let flags = checked(libc::fcntl(write, libc::F_GETFL))?;
            checked(libc::fcntl(write, libc::F_SETFL, flags | libc::O_NONBLOCK))?;
        }
        if WRITTEN_TO
            .compare_exchange(
                -1,
                write,
                std::sync::atomic::Ordering::SeqCst,
                std::sync::atomic::Ordering::SeqCst,
            )
            .is_err()
        {
            // SAFETY: both descriptors are this call's own.
            unsafe {
                libc::close(read);
                libc::close(write);
            }
            return Err(std::io::Error::from(std::io::ErrorKind::AlreadyExists));
        }
        for signal in ENDING {
            // SAFETY: the action is zeroed, then given a handler that makes
            // only calls a handler may make.
            unsafe {
                let mut action: libc::sigaction = std::mem::zeroed();
                action.sa_sigaction = caught as extern "C" fn(libc::c_int) as libc::sighandler_t;
                action.sa_flags = libc::SA_RESTART;
                libc::sigemptyset(&raw mut action.sa_mask);
                checked(libc::sigaction(
                    signal,
                    &raw const action,
                    std::ptr::null_mut(),
                ))?;
            }
        }
        // SAFETY: `read` is open and owned by nothing else.
        let read = unsafe { std::fs::File::from_raw_fd(read) };
        Ok(Self { read })
    }

    /// Waits for the next of the three signals, and gives its number.
    ///
    /// # Errors
    ///
    /// The pipe could not be read.
    pub fn wait(&self) -> std::io::Result<i32> {
        use std::io::Read;
        let mut byte = [0_u8; 1];
        (&self.read).read_exact(&mut byte)?;
        Ok(i32::from(byte[0]))
    }
}

/// Ends this process by `signal`, as its default action does, so that the
/// parent sees a process a signal ended; exits `128 + signal` if the
/// signal does not end it.
#[cfg(unix)]
pub fn end_by(signal: i32) -> ! {
    // SAFETY: restoring the default action and raising a signal touch no
    // memory of this process.
    unsafe {
        libc::signal(signal, libc::SIG_DFL);
        libc::raise(signal);
    }
    std::process::exit(128 + signal)
}

/// Whether the child `pid` of this process has ended: it is reaped now, or
/// it was before.
#[cfg(unix)]
#[must_use]
pub fn reaped(pid: u32) -> bool {
    let Ok(pid) = libc::pid_t::try_from(pid) else {
        return true;
    };
    let mut status: libc::c_int = 0;
    // SAFETY: `status` is a valid place to write; `WNOHANG` never blocks.
    let answer = unsafe { libc::waitpid(pid, &raw mut status, libc::WNOHANG) };
    answer != 0
}

#[cfg(all(test, unix))]
mod tests {
    use super::{EndingSignals, Stop, stop_group};
    use std::os::unix::process::CommandExt;
    use std::process::Command;

    #[test]
    fn a_caught_signal_is_given_and_a_child_still_ends_by_it() {
        use std::os::unix::process::ExitStatusExt;
        let signals = EndingSignals::catch().unwrap();
        assert!(EndingSignals::catch().is_err());
        let mut child = Command::new("sleep").arg("30").spawn().unwrap();
        Command::new("kill")
            .args(["-TERM", &child.id().to_string()])
            .status()
            .unwrap();
        assert_eq!(child.wait().unwrap().signal(), Some(libc::SIGTERM));
        Command::new("kill")
            .args(["-HUP", &std::process::id().to_string()])
            .status()
            .unwrap();
        assert_eq!(signals.wait().unwrap(), libc::SIGHUP);
    }

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

//! Called only during disposable-worker startup, before accepting work.
use std::{io, mem::MaybeUninit};

pub(super) fn install() -> io::Result<()> {
    // SAFETY: libc's sigaction/sigset_t consist of nullable pointers and integer
    // fields. Explicit initialization below provides a default disposition and
    // a valid empty mask before passing either object to the system.
    let mut action = unsafe { MaybeUninit::<libc::sigaction>::zeroed().assume_init() };
    let mut unblocked = unsafe { MaybeUninit::<libc::sigset_t>::zeroed().assume_init() };
    action.sa_sigaction = libc::SIG_DFL;
    // SAFETY: all pointers are to correctly sized, exclusively owned records.
    if unsafe { libc::sigemptyset(&mut action.sa_mask) } != 0
        || unsafe { libc::sigemptyset(&mut unblocked) } != 0
    {
        return Err(io::Error::last_os_error());
    }
    for signal in [libc::SIGXCPU, libc::SIGXFSZ] {
        // SAFETY: these signals permit a default disposition. No Rust callback
        // executes in a signal handler; the OS performs default termination.
        if unsafe { libc::sigaction(signal, &action, std::ptr::null_mut()) } != 0
            || unsafe { libc::sigaddset(&mut unblocked, signal) } != 0
        {
            return Err(io::Error::last_os_error());
        }
    }
    // SAFETY: unblock the limit signals on the startup thread. Future codec
    // threads inherit this mask. The worker must invoke this before media work.
    let status =
        unsafe { libc::pthread_sigmask(libc::SIG_UNBLOCK, &unblocked, std::ptr::null_mut()) };
    if status != 0 {
        return Err(io::Error::from_raw_os_error(status));
    }
    Ok(())
}

use std::io::{self, BufRead, IsTerminal, Write};

pub fn read_password(prompt: &str) -> io::Result<String> {
    let stdin = io::stdin();
    if !stdin.is_terminal() {
        let mut line = String::new();
        stdin.lock().read_line(&mut line)?;
        return Ok(line.trim_end_matches(['\r', '\n']).to_owned());
    }
    let mut stderr = io::stderr();
    write!(stderr, "{prompt}")?;
    stderr.flush()?;
    let echo = EchoGuard::disable();
    let mut line = String::new();
    let read = stdin.lock().read_line(&mut line);
    drop(echo);
    writeln!(stderr)?;
    read?;
    Ok(line.trim_end_matches(['\r', '\n']).to_owned())
}

struct EchoGuard {
    #[cfg(unix)]
    original: Option<libc::termios>,
}

impl EchoGuard {
    #[cfg(unix)]
    fn disable() -> Self {
        let mut settings = std::mem::MaybeUninit::<libc::termios>::zeroed();
        let original = unsafe {
            (libc::tcgetattr(libc::STDIN_FILENO, settings.as_mut_ptr()) == 0)
                .then(|| settings.assume_init())
        };
        if let Some(original) = original {
            let mut silent = original;
            silent.c_lflag &= !libc::ECHO;
            unsafe {
                libc::tcsetattr(libc::STDIN_FILENO, libc::TCSANOW, &silent);
            }
        }
        Self { original }
    }

    #[cfg(not(unix))]
    fn disable() -> Self {
        Self {}
    }
}

impl Drop for EchoGuard {
    fn drop(&mut self) {
        #[cfg(unix)]
        if let Some(original) = &self.original {
            unsafe {
                libc::tcsetattr(libc::STDIN_FILENO, libc::TCSANOW, original);
            }
        }
    }
}

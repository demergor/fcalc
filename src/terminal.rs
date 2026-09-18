use std::{
    error, fmt::{self, Display}, io::{self, Write}, mem::MaybeUninit,
};

pub const ESC: u8 = b'\x1b';

const ENTER_ALT_SCREEN: &str = "\x1b[?1049h";
const LEAVE_ALT_SCREEN: &str = "\x1b[?1049l";

#[derive(Debug)]
pub struct Terminal {
    pub width: u16,
    pub height: u16,
    og_termios: libc::termios,
}

impl Terminal {
    pub fn new() -> Result<Self, TerminalPreparationError> {
        #[cfg(not(any(target_os = "linux", target_os = "macos")))]
        return Err(TerminalPreparationError::UnsupportedOperatingSystem);

        let size = terminal_size()?;
        let og_termios = terminal_attributes()?;

        let mut raw = og_termios;
        raw.c_lflag &= !(libc::ECHO | libc::ICANON);
        set_terminal_attributes(&raw)?;

        let mut stdout = io::stdout();
        stdout.write_all(ENTER_ALT_SCREEN.as_bytes())?;
        stdout.write_all("\x1b[H".as_bytes())?;
        stdout.flush()?;

        Ok(Self {
            width: size.ws_col,
            height: size.ws_row,
            og_termios,
        })
    }

    pub fn byte_ready(timeout_ms: i32) -> io::Result<bool> {
        let mut pollfd = libc::pollfd {
            fd: libc::STDIN_FILENO,
            events: libc::POLLIN,
            revents: 0,
        };

        let result = unsafe { libc::poll(&mut pollfd, 1, timeout_ms) };
        if result < 0 {
            return Err(io::Error::last_os_error());
        }

        Ok(result > 0 && (pollfd.revents & libc::POLLIN) != 0)
    }
}

impl Drop for Terminal {
    fn drop(&mut self) {
        let _ = set_terminal_attributes(&self.og_termios);
        let mut stdout = io::stdout();
        let _ = stdout.write_all(LEAVE_ALT_SCREEN.as_bytes());
        let _ = stdout.flush();
    }
}

#[derive(Debug)]
pub enum TerminalPreparationError {
    #[cfg(not(any(target_os = "linux", target_os = "macos")))]
    UnsupportedOperatingSystem,
    BadTerminalSize,
    Io(io::Error),
}

impl From<io::Error> for TerminalPreparationError {
    fn from(error: io::Error) -> TerminalPreparationError {
        Self::Io(error)
    }
}

impl Display for TerminalPreparationError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            #[cfg(not(any(target_os = "linux", target_os = "macos")))]
            Self::UnsupportedOperatingSystem => {
                write!(f, "This program is not supported on your operating system!")
            }
            Self::BadTerminalSize => write!(f, "Couldn't fetch terminal size!"),
            Self::Io(err) => write!(f, "Terminal: {err}"),
        }
    }
}

impl error::Error for TerminalPreparationError {}

#[cfg(any(target_os = "linux", target_os = "macos"))]
fn terminal_attributes() -> Result<libc::termios, TerminalPreparationError> {
    let mut termios = MaybeUninit::<libc::termios>::uninit();
    let result = unsafe { libc::tcgetattr(libc::STDIN_FILENO, termios.as_mut_ptr()) };

    if result == -1 {
        return Err(TerminalPreparationError::Io(io::Error::last_os_error()));
    }

    Ok(unsafe { termios.assume_init() })
}

#[cfg(any(target_os = "linux", target_os = "macos"))]
fn set_terminal_attributes(
    termios: &libc::termios,
) -> Result<(), TerminalPreparationError> {
    let result =
        unsafe { libc::tcsetattr(libc::STDIN_FILENO, libc::TCSAFLUSH, termios) };
    if result == -1 {
        return Err(TerminalPreparationError::Io(io::Error::last_os_error()));
    }

    Ok(())
}

#[cfg(any(target_os = "linux", target_os = "macos"))]
fn terminal_size() -> Result<libc::winsize, TerminalPreparationError> {
    let mut size = MaybeUninit::<libc::winsize>::uninit();
    let result = unsafe {
        libc::ioctl(libc::STDOUT_FILENO, libc::TIOCGWINSZ, size.as_mut_ptr())
    };

    if result == -1 {
        return Err(TerminalPreparationError::Io(io::Error::last_os_error()));
    }

    let size = unsafe { size.assume_init() };
    if size.ws_col == 0 || size.ws_row == 0 {
        return Err(TerminalPreparationError::BadTerminalSize);
    }

    Ok(size)
}

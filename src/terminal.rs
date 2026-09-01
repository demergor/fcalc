use std::{
    ffi::{c_int, c_ulong},
    io::{self, Write},
    mem::MaybeUninit,
};

// Control
const _CLEAR_SCREEN: &str = "\x1b[2J"; // Clears visible screen
const _CLEAR_LINE: &str = "\x1b[2K"; // Clears current line
const _HIDE_CURSOR: &str = "\x1b[?25l";
const _SHOW_CURSOR: &str = "\x1b[?25h";
const ENTER_ALT_SCREEN: &str = "\x1b[?1049h";
const LEAVE_ALT_SCREEN: &str = "\x1b[?1049l";
const _RESET: &str = "\x1b[0m";

// Navigation
const _CURSOR_HOME: &str = "\x1b[H"; // Moves cursor to (0,0)
const _CURSOR_UP: &str = "\x1b[A";
const _CURSOR_DOWN: &str = "\x1b[B";
const _CURSOR_RIGHT: &str = "\x1b[C";
const _CURSOR_LEFT: &str = "\x1b[D";

// Styles
const _BOLD: &str = "\x1b[1m";
const _DIM: &str = "\x1b[2m";
const _UNDERLINE: &str = "\x1b[4m";

// Foreground (Text) Colors
const _GREY: &str = "\x1b[90m";
const _RED: &str = "\x1b[31m";
const _BLACK: &str = "\x1b[30m";
const _GREEN: &str = "\x1b[32m";
const _YELLOW: &str = "\x1b[33m";
const _BLUE: &str = "\x1b[34m";
const _MAGENTA: &str = "\x1b[35m";
const _CYAN: &str = "\x1b[36m";
const _WHITE: &str = "\x1b[37m";

#[repr(C)]
struct WinSize {
    ws_row: u16,
    ws_col: u16,
    ws_xpixel: u16,
    ws_ypixel: u16,
}

#[allow(rust_analyzer::inactive_code)]
#[cfg(target_os = "linux")]
#[repr(C)]
#[derive(Debug, Clone, Copy)]
struct Termios {
    c_iflag: u32,
    c_oflag: u32,
    c_cflag: u32,
    c_lflag: u32,
    c_line: u8,
    c_cc: [u8; 32],
    c_ispeed: u32,
    c_ospeed: u32,
}

#[allow(rust_analyzer::inactive_code)]
#[cfg(target_os = "macos")]
#[repr(C)]
#[derive(Debug, Clone, Copy)]
struct Termios {
    c_iflag: c_ulong,
    c_oflag: c_ulong,
    c_cflag: c_ulong,
    c_lflag: c_ulong,
    c_cc: [u8; 20],
    c_ispeed: c_ulong,
    c_ospeed: c_ulong,
}

#[allow(rust_analyzer::inactive_code)]
#[cfg(target_os = "linux")]
const TCSAFLUSH: c_int = 2;

#[allow(rust_analyzer::inactive_code)]
#[cfg(target_os = "macos")]
const TCSAFLUSH: c_int = 10;

const ECHO: u32 = 0x0000_0008;
const ICANON: u32 = 0x0000_0002;

unsafe extern "C" {
    fn tcgetattr(fd: c_int, termios_ptr: *mut Termios) -> c_int;
    fn tcsetattr(
        fd: c_int,
        optional_actions: c_int,
        termios_ptr: *const Termios,
    ) -> c_int;
}

#[derive(Debug, PartialEq, Eq)]
pub enum TerminalPreparationError {
    BadTerminalSize,
    IoctlFailure,
}

#[derive(Debug)]
pub struct Terminal {
    pub width: u16,
    pub height: u16,
    og_termios: Termios,
}

impl Terminal {
    pub fn new() -> Result<Terminal, TerminalPreparationError> {
        use TerminalPreparationError as Error;

        unsafe extern "C" {
            fn ioctl(fd: c_int, request: c_ulong, ...) -> c_int;
        }

        #[allow(rust_analyzer::inactive_code)]
        #[cfg(not(any(target_os = "linux", target_os = "macos")))]
        return Err(Error::UnsupportedOperatingSystem);

        #[allow(rust_analyzer::inactive_code)]
        #[cfg(target_os = "linux")]
        const TIOCGWINSZ: c_ulong = 0x5413;

        #[allow(rust_analyzer::inactive_code)]
        #[cfg(target_os = "macos")]
        const TIOCGWINSZ: c_ulong = 0x40087468;

        #[allow(rust_analyzer::inactive_code)]
        #[cfg(not(any(target_os = "linux", target_os = "macos")))]
        const TIOCGWINSZ: c_ulong = 0;

        let size = unsafe {
            let mut size = MaybeUninit::<WinSize>::uninit();
            if ioctl(1, TIOCGWINSZ, size.as_mut_ptr()) != 0 {
                return Err(Error::IoctlFailure);
            }

            let size = size.assume_init();
            if !(size.ws_col > 0 && size.ws_row > 0) {
                return Err(Error::BadTerminalSize);
            }

            size
        };

        let og_termios = unsafe {
            let mut termios = MaybeUninit::<Termios>::uninit();
            if tcgetattr(0, termios.as_mut_ptr()) != 0 {
                return Err(Error::IoctlFailure);
            }

            termios.assume_init()
        };

        let mut raw = og_termios;
        raw.c_lflag &= !(ECHO | ICANON);

        unsafe {
            if tcsetattr(0, TCSAFLUSH, &raw) != 0 {
                return Err(Error::IoctlFailure);
            }
        }

        print!("{ENTER_ALT_SCREEN}");
        io::stdout().flush().ok();

        Ok(Terminal {
            width: size.ws_col,
            height: size.ws_row,
            og_termios,
        })
    }
}

impl Drop for Terminal {
    fn drop(&mut self) {
        print!("{LEAVE_ALT_SCREEN}");
        io::stdout().flush().ok();

        unsafe { 
            tcsetattr(0, TCSAFLUSH, &self.og_termios) 
        };
    }
}

//! Losing the terminal — the built game on a real pseudo-terminal, with the
//! terminal taken away.
//!
//! @spec TUI-RUN-046, TUI-RUN-047, TUI-RUN-048, TUI-RUN-050, TUI-RUN-051,
//! TUI-RUN-052, TUI-RUN-057, TUI-RUN-058, TUI-RUN-059, TUI-RUN-060,
//! TUI-RUN-061, TUI-RUN-062, TUI-RUN-063, TUI-RUN-064, TUI-RUN-065
//!
//! **Mouse reporting rides the same harness.** What the game asks of the
//! terminal — the mouse reporting it turns on at startup and off on every way
//! out, the lost terminal's included — and what a pointer report sent to it
//! does are claims about the built game on a real terminal too, so the last
//! part of this file makes them here, through the same launches.
//!
//! **These tests are what holds the terminal library to its behaviour.** The
//! runtime design builds crossterm with its terminal-device input source
//! (`use-dev-tty`) because that source returns from every wait for input by
//! the time it was given, whatever has become of the terminal; the library's
//! default source reads a vanished terminal forever inside one wait, and the
//! game then runs on at full processor, unsaved, holding its session lock,
//! deaf to a termination signal. Which source is built, when its waits
//! return, and what it does at the end of input are the inside of a
//! dependency reached through one line of build configuration, and nothing in
//! the shell's own code can promise them across an upgrade. So every claim
//! here is made of the real binary: it is run on a pseudo-terminal, the
//! terminal is closed (or refuses frames, or a signal follows), and the game
//! must save, release its lock, and exit with a quit's status — in play and
//! while waking, with a hangup delivered and with none. A crossterm upgrade,
//! or a change to the workspace's dependency line, that goes back to waiting
//! forever or stops reading what is ready fails this file. Do not relax it to
//! make an upgrade pass.
//!
//! **How the game is run.** As `loop_order.rs` runs it — the real `clowder`
//! binary as a child, its save redirected by a scratch `HOME` — but on a
//! pseudo-terminal this file opens itself, with the libc calls that needs
//! declared by hand (as `runtime::term` declares the ones it needs) rather
//! than through a dependency. Every child is put in a session of its own, so
//! it can never reach the terminal the tests were started from. Two shapes of
//! terminal are used, because the operating system only sends a hangup to the
//! session a terminal controls:
//!
//! - **controlling** — the pseudo-terminal is the child's controlling
//!   terminal, so closing it delivers a hangup, as closing a tmux pane does;
//! - **not controlling** — the child merely has the terminal open, so closing
//!   it delivers no signal at all and the game has to notice by itself.
//!
//! **The harness's own rules**, each learned the hard way: it keeps reading
//! the terminal's master side while it waits for an exit (an exiting process
//! blocks until its terminal output is drained); every wait is bounded; and
//! every child is killed and reaped on every path, by `Drop`.
//!
//! **What is not asserted here.** How many times the loop asked for input
//! after the terminal went, which events it discarded, and that it drew no
//! further frame are properties of the loop's inside; `session.rs` asserts
//! them against a scripted input source and a backend that refuses writes.
//! Which of its two questions told the loop — its input or its frame — differs
//! by operating system (a vanished terminal reads as a failure on one and as
//! an end of input on another), so these tests assert the ending, not the
//! route to it.

#![cfg(any(target_os = "macos", target_os = "linux"))]

use std::ffi::CStr;
use std::fs::{self, File, OpenOptions};
use std::io::{self, Read, Write};
use std::os::fd::{AsRawFd, FromRawFd};
use std::os::raw::c_int;
use std::os::unix::fs::OpenOptionsExt;
use std::os::unix::process::{CommandExt, ExitStatusExt};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdout, Command, ExitStatus, Stdio};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Mutex;
use std::time::{Duration, Instant};

/// The binary under test.
const CLOWDER: &str = env!("CARGO_BIN_EXE_clowder");

/// How long a launch may take to draw its first frame. Generous: it bounds a
/// broken start, not a healthy one.
const START_BUDGET: Duration = Duration::from_secs(20);

/// How long the game may take to be gone once it has been given a reason to
/// go. The design's bound is a frame; this is "a few seconds", so that a
/// loaded machine does not fail the suite and a game that never leaves does.
const EXIT_BUDGET: Duration = Duration::from_secs(5);

// ---------------------------------------------------------------------------
// The libc this file needs, declared by hand
// ---------------------------------------------------------------------------

mod sys {
    use std::os::raw::{c_char, c_int, c_ulong};

    pub const O_RDWR: c_int = 2;
    pub const F_SETFD: c_int = 2;
    pub const F_GETFL: c_int = 3;
    pub const F_SETFL: c_int = 4;
    pub const FD_CLOEXEC: c_int = 1;
    pub const SIGTERM: c_int = 15;

    #[cfg(target_os = "macos")]
    mod os {
        use std::os::raw::{c_int, c_ulong};
        pub const O_NOCTTY: c_int = 0x0002_0000;
        pub const O_NONBLOCK: c_int = 0x0004;
        pub const TIOCSCTTY: c_ulong = 0x2000_7461;
        pub const TIOCSWINSZ: c_ulong = 0x8008_7467;
        pub type TcFlag = c_ulong;
        pub const ICANON: TcFlag = 0x100;
        pub const ECHO: TcFlag = 0x8;
    }

    #[cfg(target_os = "linux")]
    mod os {
        use std::os::raw::{c_int, c_uint, c_ulong};
        pub const O_NOCTTY: c_int = 0o400;
        pub const O_NONBLOCK: c_int = 0o4000;
        pub const TIOCSCTTY: c_ulong = 0x540E;
        pub const TIOCSWINSZ: c_ulong = 0x5414;
        pub type TcFlag = c_uint;
        pub const ICANON: TcFlag = 0o2;
        pub const ECHO: TcFlag = 0o10;
    }

    pub use os::*;

    #[repr(C)]
    pub struct Winsize {
        pub rows: u16,
        pub cols: u16,
        pub xpixel: u16,
        pub ypixel: u16,
    }

    /// `struct termios`, as far as this file reads it: the four mode words
    /// lead on both platforms, and the padding is larger than either
    /// platform's remainder so `tcgetattr` always has room to write.
    #[repr(C)]
    pub struct Termios {
        pub c_iflag: TcFlag,
        pub c_oflag: TcFlag,
        pub c_cflag: TcFlag,
        pub c_lflag: TcFlag,
        pub rest: [u8; 256],
    }

    extern "C" {
        pub fn posix_openpt(flags: c_int) -> c_int;
        pub fn grantpt(fd: c_int) -> c_int;
        pub fn unlockpt(fd: c_int) -> c_int;
        pub fn ptsname(fd: c_int) -> *mut c_char;
        pub fn ioctl(fd: c_int, request: c_ulong, ...) -> c_int;
        pub fn fcntl(fd: c_int, cmd: c_int, ...) -> c_int;
        pub fn setsid() -> c_int;
        pub fn kill(pid: c_int, sig: c_int) -> c_int;
        pub fn tcgetattr(fd: c_int, termios: *mut Termios) -> c_int;
    }
}

/// Held from the moment a pseudo-terminal is opened until the child that gets
/// it has been spawned. Two reasons, both about tests running in parallel in
/// this one process: `ptsname` hands back a buffer shared by every caller,
/// and a child forked by another test between `posix_openpt` and the
/// close-on-exec flag would inherit this terminal's master side — after which
/// closing it here would not take the terminal away at all.
static SPAWNING: Mutex<()> = Mutex::new(());

fn make_nonblocking(fd: c_int) {
    // SAFETY: fcntl on a descriptor the caller owns.
    unsafe {
        let flags = sys::fcntl(fd, sys::F_GETFL);
        assert!(flags >= 0, "F_GETFL: {}", io::Error::last_os_error());
        assert!(
            sys::fcntl(fd, sys::F_SETFL, flags | sys::O_NONBLOCK) >= 0,
            "F_SETFL: {}",
            io::Error::last_os_error()
        );
    }
}

/// A pseudo-terminal a hundred columns by thirty rows: (master, terminal).
/// The caller holds `SPAWNING`.
fn open_terminal() -> (File, File) {
    // SAFETY: libc calls on a descriptor this function has just been given
    // and wraps in a `File` before anything can fail.
    let (master, name) = unsafe {
        let fd = sys::posix_openpt(sys::O_RDWR | sys::O_NOCTTY);
        assert!(fd >= 0, "posix_openpt: {}", io::Error::last_os_error());
        let master = File::from_raw_fd(fd);
        assert!(
            sys::fcntl(fd, sys::F_SETFD, sys::FD_CLOEXEC) >= 0,
            "F_SETFD: {}",
            io::Error::last_os_error()
        );
        assert_eq!(
            sys::grantpt(fd),
            0,
            "grantpt: {}",
            io::Error::last_os_error()
        );
        assert_eq!(
            sys::unlockpt(fd),
            0,
            "unlockpt: {}",
            io::Error::last_os_error()
        );
        let name = sys::ptsname(fd);
        assert!(!name.is_null(), "ptsname: {}", io::Error::last_os_error());
        (master, CStr::from_ptr(name).to_string_lossy().into_owned())
    };
    let terminal = OpenOptions::new()
        .read(true)
        .write(true)
        // The test process must not take this terminal as its own.
        .custom_flags(sys::O_NOCTTY)
        .open(&name)
        .unwrap_or_else(|e| panic!("opening the terminal side {name}: {e}"));
    let size = sys::Winsize {
        rows: 30,
        cols: 100,
        xpixel: 0,
        ypixel: 0,
    };
    // SAFETY: TIOCSWINSZ reads one `winsize` from the pointer it is given.
    let set = unsafe {
        sys::ioctl(
            terminal.as_raw_fd(),
            sys::TIOCSWINSZ,
            &size as *const sys::Winsize,
        )
    };
    assert_eq!(set, 0, "TIOCSWINSZ: {}", io::Error::last_os_error());
    make_nonblocking(master.as_raw_fd());
    (master, terminal)
}

/// The terminal's local modes — whether it is cooked (canonical, echoing) or
/// raw — asked of its master side, which goes on answering after the game
/// has gone (the terminal side of a session's controlling terminal is revoked
/// when the session's leader exits).
fn local_modes(master: &File) -> (bool, bool) {
    let mut t = sys::Termios {
        c_iflag: 0,
        c_oflag: 0,
        c_cflag: 0,
        c_lflag: 0,
        rest: [0; 256],
    };
    // SAFETY: `Termios` is at least as large as the platform's `struct termios`.
    let got = unsafe { sys::tcgetattr(master.as_raw_fd(), &mut t) };
    assert_eq!(got, 0, "tcgetattr: {}", io::Error::last_os_error());
    (t.c_lflag & sys::ICANON != 0, t.c_lflag & sys::ECHO != 0)
}

// ---------------------------------------------------------------------------
// Scratch homes and fixture saves
// ---------------------------------------------------------------------------

/// Where every scratch home this file hands a child lives: inside the build's
/// own target directory, which version control ignores, and never anywhere a
/// player's real save could be.
fn scratch_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/claude-tmp")
}

/// A scratch home that cleans itself up, named uniquely per call: tests run
/// in parallel in one process.
struct TempDir {
    path: PathBuf,
}

impl TempDir {
    fn new(tag: &str) -> TempDir {
        static COUNTER: AtomicU32 = AtomicU32::new(0);
        let n = COUNTER.fetch_add(1, Ordering::Relaxed);
        let path =
            scratch_root().join(format!("clowder-lost-{}-{}-{}", std::process::id(), tag, n));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).expect("a scratch directory should be creatable");
        TempDir { path }
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

/// Where the child will look for its save, given the home it is handed — the
/// platform rule `clowder_core::default_save_path` follows.
fn child_save_path(home: &Path) -> PathBuf {
    if cfg!(target_os = "macos") {
        home.join("Library/Application Support/clowder/save.json")
    } else {
        home.join(".local/share/clowder/save.json")
    }
}

/// The ticks a save carries as owed: read through the core against a clock
/// before its stamp, which owes no interval, so the total is the carried
/// count alone.
fn owed_by(save_contents: &str) -> u64 {
    clowder_core::begin_load(save_contents, 0)
        .expect("the save loads")
        .total_ticks()
}

// ---------------------------------------------------------------------------
// The game on a pseudo-terminal
// ---------------------------------------------------------------------------

/// Whether closing the terminal delivers a hangup to the game.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Terminal {
    /// The game's controlling terminal: closing it delivers a hangup.
    Controlling,
    /// A terminal the game merely has open: closing it delivers no signal.
    NotControlling,
}

/// Where the game's frames go.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Frames {
    ToTheTerminal,
    /// To a pipe the test can close while the terminal itself lives on, so a
    /// frame is refused with the terminal still there and no signal sent.
    ToAPipe,
}

/// Where the game's parting words go.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Parting {
    /// To the terminal, as for a player: lost with it.
    ToTheTerminal,
    /// To a file the test reads afterwards.
    ToAFile,
}

/// Which state the game is in when the test acts on it.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Phase {
    /// A new game: awake from its first frame.
    Play,
    /// A save away far longer than any machine replays inside a test: waking
    /// for as long as the test lasts.
    Waking,
    /// A save of a meadow whose first section lists far more rows than the
    /// screen holds, stamped as the launch begins: it opens at once, with a
    /// long list under the cursor for the pointer to move through.
    Lively,
}

/// The meadow `Phase::Lively` saves: every structure on show and the means
/// to take any of them, so its first section lists a hundred rows and more.
fn lively() -> clowder_core::GameState {
    let mut s = clowder_core::GameState::new(60_065);
    for r in clowder_core::ResourceId::ALL {
        s.resources[r.ix()] = 1.0e6;
    }
    s.revealed = clowder_core::StructureId::ALL
        .iter()
        .map(|b| clowder_core::ContentId::Structure(*b))
        .collect();
    s
}

struct Game {
    child: Child,
    /// The terminal's master side; `None` once the terminal has been closed.
    master: Option<File>,
    /// The frames' pipe, where frames do not go to the terminal.
    pipe: Option<ChildStdout>,
    /// Everything the game has written to its screen.
    drawn: Vec<u8>,
    phase: Phase,
    /// The fixture save's contents, where there was one.
    fixture: Option<String>,
    home: TempDir,
}

impl Game {
    fn launch(
        tag: &str,
        phase: Phase,
        terminal: Terminal,
        frames: Frames,
        parting: Parting,
    ) -> Game {
        Game::launch_with(tag, phase, terminal, frames, parting, &[])
    }

    /// As `launch`, with arguments on the command line.
    fn launch_with(
        tag: &str,
        phase: Phase,
        terminal: Terminal,
        frames: Frames,
        parting: Parting,
        args: &[&str],
    ) -> Game {
        let home = TempDir::new(tag);
        let save = child_save_path(&home.path);
        let fixture = match phase {
            Phase::Play => None,
            Phase::Waking => {
                fs::create_dir_all(save.parent().unwrap()).unwrap();
                // Stamped at the epoch's first second: today is far past the
                // catch-up cap, so the load owes the whole cap.
                let json = clowder_core::save_string(&clowder_core::GameState::new(46_048), 1);
                fs::write(&save, &json).expect("writing the fixture save");
                Some(json)
            }
            Phase::Lively => {
                fs::create_dir_all(save.parent().unwrap()).unwrap();
                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .expect("the clock is past the epoch")
                    .as_secs();
                let json = clowder_core::save_string(&lively(), now);
                fs::write(&save, &json).expect("writing the fixture save");
                Some(json)
            }
        };

        let spawning = SPAWNING.lock().unwrap_or_else(|e| e.into_inner());
        let (master, term) = open_terminal();
        let mut command = Command::new(CLOWDER);
        command
            .env_clear()
            .env("HOME", &home.path)
            .env("XDG_DATA_HOME", home.path.join(".local/share"))
            .env("PATH", "/usr/bin:/bin")
            .env("TERM", "xterm-256color")
            .env("COLORTERM", "truecolor")
            .current_dir(&home.path)
            .args(args)
            .stdin(Stdio::from(
                term.try_clone().expect("duplicating the terminal"),
            ));
        match frames {
            Frames::ToTheTerminal => command.stdout(Stdio::from(term.try_clone().unwrap())),
            Frames::ToAPipe => command.stdout(Stdio::piped()),
        };
        match parting {
            Parting::ToTheTerminal => command.stderr(Stdio::from(term.try_clone().unwrap())),
            Parting::ToAFile => command.stderr(Stdio::from(
                File::create(home.path.join("stderr.txt")).expect("creating the stderr file"),
            )),
        };
        let controlling = terminal == Terminal::Controlling;
        // SAFETY: the closure runs between fork and exec and makes only
        // async-signal-safe calls. A session of its own puts the child out of
        // reach of the terminal the tests were started from; standard input
        // is already the pseudo-terminal by the time it runs.
        unsafe {
            command.pre_exec(move || {
                if sys::setsid() < 0 {
                    return Err(io::Error::last_os_error());
                }
                let no_stealing: c_int = 0;
                if controlling && sys::ioctl(0, sys::TIOCSCTTY, no_stealing) < 0 {
                    return Err(io::Error::last_os_error());
                }
                Ok(())
            });
        }
        let mut child = command
            .spawn()
            .unwrap_or_else(|e| panic!("spawning {CLOWDER}: {e}"));
        // The command holds copies of the terminal side; with these gone the
        // child's are the only ones.
        drop(command);
        drop(term);
        drop(spawning);

        let pipe = child.stdout.take();
        if let Some(pipe) = &pipe {
            make_nonblocking(pipe.as_raw_fd());
        }
        let mut game = Game {
            child,
            master: Some(master),
            pipe,
            drawn: Vec::new(),
            phase,
            fixture,
            home,
        };
        game.wait_for_the_first_frame();
        game
    }

    fn save_path(&self) -> PathBuf {
        child_save_path(&self.home.path)
    }

    /// What sits beside the save, other than the save.
    fn beside_the_save(&self) -> Vec<String> {
        let save = self.save_path();
        let mut names: Vec<String> = fs::read_dir(save.parent().unwrap())
            .map(|rd| {
                rd.flatten()
                    .map(|e| e.file_name().to_string_lossy().into_owned())
                    .collect()
            })
            .unwrap_or_default();
        names.retain(|n| Some(n.as_str()) != save.file_name().and_then(|s| s.to_str()));
        names.sort();
        names
    }

    fn locks(&self) -> Vec<String> {
        self.beside_the_save()
            .into_iter()
            .filter(|n| n.contains("lock"))
            .collect()
    }

    /// Read whatever the game has written, without waiting.
    fn drain(&mut self) {
        let mut buf = [0u8; 16 * 1024];
        if let Some(master) = self.master.as_mut() {
            loop {
                match master.read(&mut buf) {
                    Ok(0) => break,
                    Ok(n) if self.pipe.is_none() => self.drawn.extend_from_slice(&buf[..n]),
                    Ok(_) => {}
                    Err(e) if e.kind() == io::ErrorKind::Interrupted => {}
                    Err(_) => break,
                }
            }
        }
        if let Some(pipe) = self.pipe.as_mut() {
            loop {
                match pipe.read(&mut buf) {
                    Ok(0) => break,
                    Ok(n) => self.drawn.extend_from_slice(&buf[..n]),
                    Err(e) if e.kind() == io::ErrorKind::Interrupted => {}
                    Err(_) => break,
                }
            }
        }
    }

    /// The game has taken the terminal and finished drawing a frame: the
    /// alternate screen entered, and after it the cursor hidden, which the
    /// drawing library does at the end of each frame. Then a moment more, so
    /// the loop is in its ordinary stride — its handlers installed, its lock
    /// held — when the test acts.
    fn wait_for_the_first_frame(&mut self) {
        const ENTER: &[u8] = b"\x1b[?1049h";
        const HIDE: &[u8] = b"\x1b[?25l";
        let find = |hay: &[u8], needle: &[u8]| hay.windows(needle.len()).position(|w| w == needle);
        let deadline = Instant::now() + START_BUDGET;
        loop {
            self.drain();
            let framed =
                find(&self.drawn, ENTER).is_some_and(|at| find(&self.drawn[at..], HIDE).is_some());
            if framed {
                break;
            }
            if let Some(status) = self.child.try_wait().expect("polling the child") {
                panic!(
                    "the game ended ({status}) before it drew a frame.\n--- stderr ---\n{}",
                    self.stderr()
                );
            }
            assert!(
                Instant::now() < deadline,
                "the game drew no frame within {START_BUDGET:?} of starting"
            );
            std::thread::sleep(Duration::from_millis(5));
        }
        let settle = Instant::now() + Duration::from_millis(250);
        while Instant::now() < settle {
            self.drain();
            std::thread::sleep(Duration::from_millis(5));
        }
        assert!(
            self.child.try_wait().expect("polling the child").is_none(),
            "fixture: the game should still be running after its first frame.\n--- stderr ---\n{}",
            self.stderr()
        );
        assert_eq!(
            self.locks().len(),
            1,
            "fixture: a running game holds one session lock beside its save; found {:?}",
            self.beside_the_save()
        );
    }

    /// Take the terminal away: close its master side, as a killed pane, a
    /// closed window, or a dropped connection does.
    fn close_the_terminal(&mut self) {
        drop(self.master.take().expect("the terminal is still open"));
    }

    /// Refuse every further frame while the terminal lives on.
    fn refuse_frames(&mut self) {
        drop(self.pipe.take().expect("fixture: frames go to a pipe"));
    }

    fn press(&mut self, keys: &[u8]) {
        self.master
            .as_mut()
            .expect("the terminal is still open")
            .write_all(keys)
            .expect("typing at the game");
    }

    fn terminate(&mut self) {
        // SAFETY: a signal to a child this struct owns and has not reaped.
        let sent = unsafe { sys::kill(self.child.id() as c_int, sys::SIGTERM) };
        assert_eq!(sent, 0, "SIGTERM: {}", io::Error::last_os_error());
    }

    /// Put something in the save's place that no write can replace: a
    /// directory with something in it. The game read its save at startup, so
    /// only the shutdown's write meets it.
    fn make_the_save_unwritable(&mut self) {
        let save = self.save_path();
        let _ = fs::remove_file(&save);
        fs::create_dir_all(&save).expect("putting a directory in the save's place");
        fs::write(save.join("in-the-way"), b"").expect("filling the directory in the save's place");
    }

    fn stderr(&self) -> String {
        fs::read_to_string(self.home.path.join("stderr.txt")).unwrap_or_default()
    }

    /// Wait, reading the terminal all the while, for the game to be gone.
    fn wait_for_exit(&mut self, budget: Duration) -> Option<(ExitStatus, Duration)> {
        let from = Instant::now();
        loop {
            self.drain();
            if let Some(status) = self.child.try_wait().expect("polling the child") {
                // Whatever it wrote on its way out.
                self.drain();
                return Some((status, from.elapsed()));
            }
            if from.elapsed() >= budget {
                return None;
            }
            std::thread::sleep(Duration::from_millis(5));
        }
    }

    /// The game is gone within the budget, by returning as a quit returns:
    /// not killed by a signal, not a panic, status zero.
    fn expect_a_quits_exit(&mut self, what: &str) {
        match self.wait_for_exit(EXIT_BUDGET) {
            None => panic!(
                "{what}: the game was still running {EXIT_BUDGET:?} later — a session nobody can \
                 see or reach, holding its save.\n--- beside the save ---\n{:?}\n--- stderr ---\n{}",
                self.beside_the_save(),
                self.stderr()
            ),
            Some((status, _)) => {
                assert_eq!(
                    status.signal(),
                    None,
                    "{what}: the game was ended by a signal rather than taking its shutdown path"
                );
                assert_eq!(
                    status.code(),
                    Some(0),
                    "{what}: the game did not exit with the status a quit has — a failed print or \
                     a failed restore on the way out was treated as an error.\n--- stderr ---\n{}",
                    self.stderr()
                );
            }
        }
        assert!(
            !self.stderr().contains("panicked at"),
            "{what}: the game panicked on its way out.\n{}",
            self.stderr()
        );
    }

    /// The session lock is gone from beside the save.
    fn expect_no_lock(&self, what: &str) {
        assert_eq!(
            self.locks(),
            Vec::<String>::new(),
            "{what}: the session lock was left beside the save, so the next launch is refused"
        );
    }

    /// The shutdown wrote a save: in play, one that loads where there was
    /// none; while waking, one that replaced the fixture and still owes the
    /// part of the absence the wake had not replayed, and no more than that.
    fn expect_the_shutdown_save(&self, what: &str) {
        let contents = fs::read_to_string(self.save_path())
            .unwrap_or_else(|e| panic!("{what}: the game left no save behind ({e})"));
        let owed = owed_by(&contents);
        match self.phase {
            Phase::Play | Phase::Lively => {}
            Phase::Waking => {
                let fixture = self.fixture.as_ref().unwrap();
                assert!(
                    &contents != fixture,
                    "{what}: the save is the one the wake started from — nothing was saved on the way out"
                );
                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .expect("the clock is past the epoch")
                    .as_secs();
                let whole = clowder_core::begin_load(fixture, now)
                    .expect("the fixture loads")
                    .total_ticks();
                assert!(
                    owed > 0,
                    "{what}: a wake cut short must leave its save owing the rest of the absence"
                );
                assert!(
                    owed < whole,
                    "{what}: the save owes the whole absence ({owed} of {whole}) — what the wake had \
                     already replayed was thrown away"
                );
            }
        }
    }

    /// Everything a clean ending leaves: gone, by a quit's status, saved, and
    /// unlocked.
    fn expect_a_clean_ending(&mut self, what: &str) {
        self.expect_a_quits_exit(what);
        self.expect_the_shutdown_save(what);
        self.expect_no_lock(what);
    }
}

impl Drop for Game {
    /// Kill and reap on every path. The terminal is closed first: a dying
    /// process waits for its terminal output to drain, and nothing is reading
    /// it any more.
    fn drop(&mut self) {
        let _ = self.child.kill();
        self.master.take();
        self.pipe.take();
        let _ = self.child.wait();
    }
}

// ---------------------------------------------------------------------------
// TUI-RUN-058 — the input source the library is built with
// ---------------------------------------------------------------------------

/// The one line of build configuration everything else here depends on: the
/// workspace asks crossterm for its terminal-device input source.
/// @spec TUI-RUN-058
#[test]
fn the_terminal_library_is_built_with_its_terminal_device_input_source() {
    let crate_manifest = Path::new(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml");
    let workspace_manifest = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../Cargo.toml");
    let asked = [&crate_manifest, &workspace_manifest]
        .into_iter()
        .any(|path| {
            let text = fs::read_to_string(path)
                .unwrap_or_else(|e| panic!("reading {}: {e}", path.display()));
            text.lines().any(|line| {
                let line = line.trim_start();
                line.starts_with("crossterm")
                    && !line.starts_with('#')
                    && line.contains("\"use-dev-tty\"")
            })
        });
    assert!(
        asked,
        "neither {} nor {} asks crossterm for its `use-dev-tty` feature, so the game is built \
         with the library's default input source — the one that never returns from a wait once \
         its terminal has gone",
        crate_manifest.display(),
        workspace_manifest.display()
    );
}

/// The terminal goes away in play — a pane killed, a window closed, a
/// connection dropped — and the game saves, releases its lock, and exits as a
/// quit exits, whether or not the operating system sends a hangup with it.
/// The parting words go to the lost terminal too, so a print that treats a
/// failed write as fatal shows here as a status that is not a quit's.
///
/// This is the regression test for the orphan that ran for weeks at a full
/// processor holding the save.
/// @spec TUI-RUN-058, TUI-RUN-046, TUI-RUN-047, TUI-RUN-052
#[test]
fn closing_the_terminal_in_play_saves_releases_the_lock_and_exits() {
    for terminal in [Terminal::NotControlling, Terminal::Controlling] {
        let what = format!("terminal closed in play ({terminal:?})");
        let mut game = Game::launch(
            "close-play",
            Phase::Play,
            terminal,
            Frames::ToTheTerminal,
            Parting::ToTheTerminal,
        );
        assert!(
            !game.save_path().exists(),
            "fixture: a new game has no save until it shuts down"
        );
        game.close_the_terminal();
        game.expect_a_clean_ending(&what);
    }
}

/// A termination signal sent once the terminal has already closed still ends
/// the session — the loop is never held from a recorded signal — and the two
/// together, in either order, are one shutdown: saved, unlocked, and gone
/// with a quit's status rather than by the signal.
/// @spec TUI-RUN-058, TUI-RUN-051
#[test]
fn a_termination_signal_after_the_terminal_has_closed_still_ends_the_session() {
    for terminal in [Terminal::NotControlling, Terminal::Controlling] {
        for signal_first in [false, true] {
            let order = if signal_first {
                "signal, then terminal closed"
            } else {
                "terminal closed, then signal"
            };
            let what = format!("{order}, in play ({terminal:?})");
            let mut game = Game::launch(
                "close-term",
                Phase::Play,
                terminal,
                Frames::ToTheTerminal,
                Parting::ToTheTerminal,
            );
            if signal_first {
                game.terminate();
                game.close_the_terminal();
            } else {
                game.close_the_terminal();
                game.terminate();
            }
            game.expect_a_clean_ending(&what);
        }
    }
}

/// The plain quit key still reaches a game in play through the input source
/// the library is built with.
/// @spec TUI-RUN-058
#[test]
fn the_quit_key_reaches_a_game_in_play() {
    for terminal in [Terminal::NotControlling, Terminal::Controlling] {
        let what = format!("q pressed in play ({terminal:?})");
        let mut game = Game::launch(
            "quit-play",
            Phase::Play,
            terminal,
            Frames::ToTheTerminal,
            Parting::ToAFile,
        );
        game.press(b"q");
        game.expect_a_clean_ending(&what);
    }
}

// ---------------------------------------------------------------------------
// TUI-RUN-048 — the terminal lost while waking
// ---------------------------------------------------------------------------

/// The terminal goes away mid-wake: the catch-up is set down as it stands and
/// saved — the save owing the rest of the absence, what was already replayed
/// kept — then the lock is released and the game exits, hangup or no hangup.
/// @spec TUI-RUN-048, TUI-RUN-058, TUI-RUN-052
#[test]
fn closing_the_terminal_while_waking_saves_where_the_wake_had_reached_and_exits() {
    for terminal in [Terminal::NotControlling, Terminal::Controlling] {
        let what = format!("terminal closed while waking ({terminal:?})");
        let mut game = Game::launch(
            "close-waking",
            Phase::Waking,
            terminal,
            Frames::ToTheTerminal,
            Parting::ToTheTerminal,
        );
        game.close_the_terminal();
        game.expect_a_clean_ending(&what);
    }
}

/// A termination signal and the loss of the terminal while waking, in either
/// order, are one shutdown too.
/// @spec TUI-RUN-051, TUI-RUN-048
#[test]
fn a_termination_signal_and_a_lost_terminal_while_waking_are_one_shutdown() {
    for signal_first in [false, true] {
        let order = if signal_first {
            "signal, then terminal closed"
        } else {
            "terminal closed, then signal"
        };
        let what = format!("{order}, while waking");
        let mut game = Game::launch(
            "close-term-waking",
            Phase::Waking,
            Terminal::NotControlling,
            Frames::ToTheTerminal,
            Parting::ToTheTerminal,
        );
        if signal_first {
            game.terminate();
            game.close_the_terminal();
        } else {
            game.close_the_terminal();
            game.terminate();
        }
        game.expect_a_clean_ending(&what);
    }
}

// ---------------------------------------------------------------------------
// TUI-RUN-059 — the briefest wait still reads what is ready
// ---------------------------------------------------------------------------

/// A waking game hears its quit key. The input source reads nothing when
/// asked to wait no time at all, and a waking frame never waits out the frame
/// budget, so this holds only while the wait a waking frame asks for is
/// longer than none.
/// @spec TUI-RUN-059
#[test]
fn a_waking_game_answers_its_quit_key() {
    for terminal in [Terminal::NotControlling, Terminal::Controlling] {
        let what = format!("q pressed while waking ({terminal:?})");
        let mut game = Game::launch(
            "quit-waking",
            Phase::Waking,
            terminal,
            Frames::ToTheTerminal,
            Parting::ToAFile,
        );
        game.press(b"q");
        game.expect_a_clean_ending(&what);
    }
}

// ---------------------------------------------------------------------------
// TUI-RUN-050 — promptly
// ---------------------------------------------------------------------------

/// A termination signal ends a session whose terminal is still there within
/// the exit budget, in play and while waking: the wait for input it arrives
/// during hands control back, and the shutdown waits for nothing.
/// @spec TUI-RUN-050
#[test]
fn a_termination_signal_ends_a_session_promptly_in_play_and_while_waking() {
    for phase in [Phase::Play, Phase::Waking] {
        let what = format!("SIGTERM with the terminal still there ({phase:?})");
        let mut game = Game::launch(
            "term",
            phase,
            Terminal::Controlling,
            Frames::ToTheTerminal,
            Parting::ToAFile,
        );
        game.terminate();
        game.expect_a_clean_ending(&what);
    }
}

// ---------------------------------------------------------------------------
// TUI-RUN-047 — a frame that cannot be written
// ---------------------------------------------------------------------------

/// The screen refuses frames while the terminal itself lives on, so nothing
/// hangs up and the input stays quiet: the refused frame alone tells the loop,
/// in play and while waking, and it ends as for any lost terminal. Here the
/// terminal can still be asked, so the restoration the shutdown attempts is
/// visible — the terminal is cooked again once the game has gone.
/// @spec TUI-RUN-047, TUI-RUN-048, TUI-RUN-052
#[test]
fn a_screen_that_refuses_frames_ends_the_session_though_no_signal_is_sent() {
    for phase in [Phase::Play, Phase::Waking] {
        let what = format!("frames refused, terminal still there ({phase:?})");
        let mut game = Game::launch(
            "refused",
            phase,
            Terminal::Controlling,
            Frames::ToAPipe,
            Parting::ToAFile,
        );
        let raw = local_modes(game.master.as_ref().unwrap());
        assert_eq!(
            raw,
            (false, false),
            "fixture: a running game holds its terminal in raw mode"
        );

        game.refuse_frames();
        game.expect_a_clean_ending(&what);

        let after = local_modes(game.master.as_ref().unwrap());
        assert_eq!(
            after,
            (true, true),
            "{what}: the terminal was left in raw mode — the shutdown did not attempt its restoration"
        );
    }
}

/// The terminal is lost while the game's frames go somewhere that still
/// accepts them, so what the shutdown writes on its way out can be read: the
/// screen is handed back exactly once — the restoration is attempted, and not
/// attempted again — though the terminal it would restore has gone.
/// @spec TUI-RUN-052
#[test]
fn the_restoration_is_attempted_once_though_the_terminal_has_gone() {
    const LEAVE: &[u8] = b"\x1b[?1049l";
    for phase in [Phase::Play, Phase::Waking] {
        let what = format!("terminal closed, frames still accepted ({phase:?})");
        let mut game = Game::launch(
            "restored-once",
            phase,
            Terminal::Controlling,
            Frames::ToAPipe,
            Parting::ToAFile,
        );
        let left_before = game
            .drawn
            .windows(LEAVE.len())
            .filter(|w| *w == LEAVE)
            .count();
        assert_eq!(
            left_before, 0,
            "fixture: a running game has not left the alternate screen"
        );

        game.close_the_terminal();
        game.expect_a_clean_ending(&what);

        let left = game
            .drawn
            .windows(LEAVE.len())
            .filter(|w| *w == LEAVE)
            .count();
        assert_eq!(
            left, 1,
            "{what}: the shutdown should hand the screen back once — it left the alternate screen \
             {left} times"
        );
    }
}

// ---------------------------------------------------------------------------
// TUI-RUN-057 — a shutdown save that cannot be written
// ---------------------------------------------------------------------------

/// What began the shutdown.
#[derive(Clone, Copy, Debug)]
enum Ending {
    QuitKey,
    Signal,
    LostTerminal,
}

/// Whatever began the shutdown, in play or while waking, a save that cannot
/// be written is tried once: the failure is named in one parting line, the
/// lock is released, and the game exits with a quit's status. It does not
/// stay to wait for the disk.
/// @spec TUI-RUN-057
#[test]
fn a_shutdown_save_that_cannot_be_written_is_tried_once_and_the_session_still_ends() {
    for phase in [Phase::Play, Phase::Waking] {
        for ending in [Ending::QuitKey, Ending::Signal, Ending::LostTerminal] {
            let what = format!("unwritable save, {ending:?}, {phase:?}");
            let mut game = Game::launch(
                "unwritable",
                phase,
                Terminal::NotControlling,
                Frames::ToTheTerminal,
                Parting::ToAFile,
            );
            game.make_the_save_unwritable();
            match ending {
                Ending::QuitKey => game.press(b"q"),
                Ending::Signal => game.terminate(),
                Ending::LostTerminal => game.close_the_terminal(),
            }
            game.expect_a_quits_exit(&what);
            game.expect_no_lock(&what);

            let stderr = game.stderr();
            let named: Vec<&str> = stderr
                .lines()
                .filter(|l| l.to_lowercase().contains("save"))
                .collect();
            assert_eq!(
                named.len(),
                1,
                "{what}: the failed save should be named in exactly one parting line — none means \
                 it went unreported, more means it was tried again.\n--- stderr ---\n{stderr}"
            );
            assert!(
                game.save_path().is_dir(),
                "fixture: what stood in the save's place is still there, so the write did fail"
            );
        }
    }
}

/// The failed save's parting line has nowhere to go when the terminal it
/// would be printed on is the one that was lost. That is nothing to report:
/// the lock is released and the exit is still a quit's.
/// @spec TUI-RUN-052, TUI-RUN-057
#[test]
fn a_parting_line_with_no_terminal_left_to_print_it_on_is_not_an_error() {
    for phase in [Phase::Play, Phase::Waking] {
        for terminal in [Terminal::NotControlling, Terminal::Controlling] {
            let what = format!("unwritable save and a lost terminal ({phase:?}, {terminal:?})");
            let mut game = Game::launch(
                "unsaid",
                phase,
                terminal,
                Frames::ToTheTerminal,
                Parting::ToTheTerminal,
            );
            game.make_the_save_unwritable();
            game.close_the_terminal();
            game.expect_a_quits_exit(&what);
            game.expect_no_lock(&what);
        }
    }
}

// ---------------------------------------------------------------------------
// Mouse reporting — TUI-RUN-060 to TUI-RUN-065, on the built game
// ---------------------------------------------------------------------------
//
// What the game asks of the terminal is read off its output as the DEC
// private modes it sets and resets (`CSI ? Pn h`, `CSI ? Pn l`), in the order
// it wrote them. What it drew is read off the same output through a small
// model of a screen: enough of a terminal to place each character the drawing
// library writes — cursor positioning, erasing the display, the characters
// themselves — and to note every place the cursor glyph was written.

/// One DEC private mode set or reset, and where in the output it was written.
#[derive(Clone, Copy, Debug, PartialEq)]
struct ModeChange {
    at: usize,
    mode: u32,
    on: bool,
}

/// The mouse modes and the alternate screen as the output sets and resets
/// them, for a failure message: the cursor's showing and hiding left out.
fn screen_and_mouse_modes(out: &[u8]) -> Vec<ModeChange> {
    mode_changes(out)
        .into_iter()
        .filter(|c| c.mode != 25)
        .collect()
}

/// Every DEC private mode the output sets or resets, in the order written.
fn mode_changes(out: &[u8]) -> Vec<ModeChange> {
    let mut changes = Vec::new();
    let mut i = 0;
    while i + 2 < out.len() {
        if out[i] == 0x1b && out[i + 1] == b'[' && out[i + 2] == b'?' {
            let mut j = i + 3;
            while j < out.len() && (out[j].is_ascii_digit() || out[j] == b';') {
                j += 1;
            }
            if j < out.len() && (out[j] == b'h' || out[j] == b'l') {
                let params = std::str::from_utf8(&out[i + 3..j]).expect("digits and semicolons");
                for p in params.split(';') {
                    if let Ok(mode) = p.parse() {
                        changes.push(ModeChange {
                            at: i,
                            mode,
                            on: out[j] == b'h',
                        });
                    }
                }
                i = j + 1;
                continue;
            }
        }
        i += 1;
    }
    changes
}

/// Normal tracking: each button's press and release, and each wheel step.
const NORMAL_TRACKING: u32 = 1000;
/// The SGR encoding of a report.
const SGR_ENCODING: u32 = 1006;
/// The alternate screen.
const ALTERNATE_SCREEN: u32 = 1049;
/// Every mouse mode a terminal knows other than the two the design asks for:
/// X10 compatibility, highlight tracking, button-motion and any-motion
/// tracking, and the UTF-8, urxvt and SGR-pixel encodings.
const UNASKED_MOUSE_MODES: [u32; 7] = [9, 1001, 1002, 1003, 1005, 1015, 1016];

/// Where the output first finishes a frame after entering the alternate
/// screen: the drawing library hides the cursor at the end of each frame.
fn first_frame_finished(out: &[u8]) -> usize {
    const HIDE: &[u8] = b"\x1b[?25l";
    let entered = mode_changes(out)
        .into_iter()
        .find(|c| c.mode == ALTERNATE_SCREEN && c.on)
        .expect("the game entered the alternate screen")
        .at;
    entered
        + out[entered..]
            .windows(HIDE.len())
            .position(|w| w == HIDE)
            .expect("the game finished a frame")
}

/// At startup the game asked for normal tracking and the SGR encoding, each
/// once, after entering the alternate screen and before its first frame was
/// finished — and, in all it wrote, set or reset no other mouse mode.
fn expect_reporting_turned_on(out: &[u8], what: &str) {
    let changes = mode_changes(out);
    let entered = changes
        .iter()
        .find(|c| c.mode == ALTERNATE_SCREEN && c.on)
        .unwrap_or_else(|| panic!("{what}: the game never entered the alternate screen"))
        .at;
    let framed = first_frame_finished(out);
    for mode in [NORMAL_TRACKING, SGR_ENCODING] {
        let on: Vec<&ModeChange> = changes.iter().filter(|c| c.mode == mode && c.on).collect();
        assert_eq!(
            on.len(),
            1,
            "{what}: mode {mode} should be turned on exactly once, at startup — it was turned on \
             {} times (mode changes: {:?})",
            on.len(),
            screen_and_mouse_modes(out)
        );
        assert!(
            on[0].at > entered && on[0].at < framed,
            "{what}: mode {mode} should be turned on with the rest of the terminal — after the \
             alternate screen is entered and before the first frame is finished"
        );
    }
    for mode in UNASKED_MOUSE_MODES {
        let touched: Vec<&ModeChange> = changes.iter().filter(|c| c.mode == mode).collect();
        assert!(
            touched.is_empty(),
            "{what}: the game set or reset mouse mode {mode}, which the design never asks for: \
             {touched:?}"
        );
    }
}

/// From `from` on — the moment the test gave the game its reason to end —
/// the game turned reporting off once, the SGR encoding first and normal
/// tracking after it, both before it left the alternate screen; it turned
/// neither off before `from`; and it turned nothing back on afterwards.
fn expect_reporting_turned_off(out: &[u8], from: usize, what: &str) {
    let changes = mode_changes(out);
    for mode in [NORMAL_TRACKING, SGR_ENCODING] {
        let early: Vec<&ModeChange> = changes
            .iter()
            .filter(|c| c.mode == mode && !c.on && c.at < from)
            .collect();
        assert!(
            early.is_empty(),
            "{what}: mode {mode} was turned off while the game was still running: {early:?}"
        );
    }
    let after: Vec<&ModeChange> = changes.iter().filter(|c| c.at >= from).collect();
    let off = |mode: u32| -> Vec<usize> {
        after
            .iter()
            .filter(|c| c.mode == mode && !c.on)
            .map(|c| c.at)
            .collect()
    };
    let sgr_off = off(SGR_ENCODING);
    let normal_off = off(NORMAL_TRACKING);
    assert_eq!(
        (sgr_off.len(), normal_off.len()),
        (1, 1),
        "{what}: on the way out the SGR encoding and normal tracking should each be turned off \
         exactly once — so the shell the terminal returns to is sent no mouse report — and they \
         were turned off {} and {} times (mode changes after the ending began: {after:?})",
        sgr_off.len(),
        normal_off.len()
    );
    assert!(
        sgr_off[0] < normal_off[0],
        "{what}: the SGR encoding is turned off before normal tracking, the reverse of the order \
         they were turned on in"
    );
    let left = after
        .iter()
        .find(|c| c.mode == ALTERNATE_SCREEN && !c.on)
        .unwrap_or_else(|| panic!("{what}: the game never left the alternate screen ({after:?})"))
        .at;
    assert!(
        normal_off[0] < left,
        "{what}: mouse reporting is turned off before the alternate screen is left, not after"
    );
    let back_on: Vec<&&ModeChange> = after
        .iter()
        .filter(|c| c.on && (c.mode == NORMAL_TRACKING || c.mode == SGR_ENCODING))
        .collect();
    assert!(
        back_on.is_empty(),
        "{what}: reporting was turned back on during the shutdown: {back_on:?}"
    );
}

/// A report in the SGR encoding: `button` as the terminal numbers it (0 the
/// left button, 64 a wheel step up, 65 a wheel step down), at the zero-based
/// cell `(x, y)`, a press (`M`) or a release (`m`).
fn sgr(button: u8, x: u16, y: u16, press: bool) -> Vec<u8> {
    format!(
        "\x1b[<{button};{};{}{}",
        x + 1,
        y + 1,
        if press { 'M' } else { 'm' }
    )
    .into_bytes()
}

fn left_click(x: u16, y: u16) -> Vec<u8> {
    let mut out = sgr(0, x, y, true);
    out.extend(sgr(0, x, y, false));
    out
}

fn wheel_down_at(x: u16, y: u16) -> Vec<u8> {
    sgr(65, x, y, true)
}

fn wheel_up_at(x: u16, y: u16) -> Vec<u8> {
    sgr(64, x, y, true)
}

/// The glyph a list draws at the start of the line the cursor is on.
const CURSOR_GLYPH: char = '\u{25B8}';

/// A screen as the game's writes leave it.
struct Glass {
    w: u16,
    h: u16,
    cells: Vec<char>,
    /// Every write of the cursor glyph: where in the output, and the cell.
    glyphs: Vec<(usize, u16, u16)>,
}

impl Glass {
    fn of(out: &[u8], (w, h): (u16, u16)) -> Glass {
        let mut g = Glass {
            w,
            h,
            cells: vec![' '; w as usize * h as usize],
            glyphs: Vec::new(),
        };
        let (mut x, mut y) = (0u16, 0u16);
        let mut i = 0;
        while i < out.len() {
            let b = out[i];
            if b == 0x1b {
                match out.get(i + 1) {
                    Some(b'[') => {
                        let mut j = i + 2;
                        while j < out.len() && (0x30..=0x3f).contains(&out[j]) {
                            j += 1;
                        }
                        let params = std::str::from_utf8(&out[i + 2..j]).unwrap_or("");
                        while j < out.len() && (0x20..=0x2f).contains(&out[j]) {
                            j += 1;
                        }
                        let Some(&fin) = out.get(j) else { break };
                        match fin {
                            b'H' | b'f' => {
                                let mut ps = params
                                    .split(';')
                                    .map(|p| p.parse::<u16>().unwrap_or(1).max(1));
                                y = ps.next().unwrap_or(1) - 1;
                                x = ps.next().unwrap_or(1) - 1;
                            }
                            b'J' if params == "2" || params == "3" => {
                                g.cells.iter_mut().for_each(|c| *c = ' ')
                            }
                            _ => {}
                        }
                        i = j + 1;
                    }
                    Some(b']') => {
                        let mut j = i + 2;
                        while j < out.len()
                            && out[j] != 0x07
                            && !(out[j] == 0x1b && out.get(j + 1) == Some(&b'\\'))
                        {
                            j += 1;
                        }
                        i = j + if out.get(j) == Some(&0x07) { 1 } else { 2 };
                    }
                    Some(_) => i += 2,
                    None => break,
                }
                continue;
            }
            if b < 0x20 || b == 0x7f {
                match b {
                    b'\r' => x = 0,
                    b'\n' => y = y.saturating_add(1),
                    0x08 => x = x.saturating_sub(1),
                    _ => {}
                }
                i += 1;
                continue;
            }
            let len = match b {
                0x00..=0x7f => 1,
                0xc0..=0xdf => 2,
                0xe0..=0xef => 3,
                _ => 4,
            };
            let Some(bytes) = out.get(i..i + len) else {
                break;
            };
            let ch = std::str::from_utf8(bytes)
                .ok()
                .and_then(|s| s.chars().next())
                .unwrap_or('?');
            if x < w && y < h {
                g.cells[y as usize * w as usize + x as usize] = ch;
                if ch == CURSOR_GLYPH {
                    g.glyphs.push((i, x, y));
                }
            }
            x = x.saturating_add(1);
            i += len;
        }
        g
    }

    fn line(&self, y: u16) -> String {
        self.cells[y as usize * self.w as usize..(y as usize + 1) * self.w as usize]
            .iter()
            .collect()
    }

    /// Every cell now showing the cursor glyph.
    fn cursor_cells(&self) -> Vec<(u16, u16)> {
        (0..self.h)
            .flat_map(|y| (0..self.w).map(move |x| (x, y)))
            .filter(|(x, y)| {
                self.cells[*y as usize * self.w as usize + *x as usize] == CURSOR_GLYPH
            })
            .collect()
    }
}

/// The size `open_terminal` gives every pseudo-terminal.
const PTY_SIZE: (u16, u16) = (100, 30);

impl Game {
    /// Write to the terminal as a player's terminal would, a little at a time
    /// where the terminal's input is full, reading its output all the while.
    fn send(&mut self, bytes: &[u8]) {
        let deadline = Instant::now() + EXIT_BUDGET;
        let mut sent = 0;
        while sent < bytes.len() {
            let master = self.master.as_mut().expect("the terminal is still open");
            match master.write(&bytes[sent..]) {
                Ok(n) => sent += n,
                Err(e)
                    if e.kind() == io::ErrorKind::WouldBlock
                        || e.kind() == io::ErrorKind::Interrupted =>
                {
                    assert!(
                        Instant::now() < deadline,
                        "the game stopped reading its terminal"
                    );
                    self.drain();
                    std::thread::sleep(Duration::from_millis(1));
                }
                Err(e) => panic!("writing to the game's terminal: {e}"),
            }
        }
    }

    fn glass(&self) -> Glass {
        Glass::of(&self.drawn, PTY_SIZE)
    }

    /// Read the terminal until `done` holds of what it shows, or the budget
    /// runs out; whether it came to hold.
    fn wait_for(&mut self, budget: Duration, done: impl Fn(&Glass) -> bool) -> bool {
        let deadline = Instant::now() + budget;
        loop {
            self.drain();
            if done(&self.glass()) {
                return true;
            }
            if Instant::now() >= deadline {
                return false;
            }
            std::thread::sleep(Duration::from_millis(5));
        }
    }

    /// Read the terminal for a while, so whatever the game was going to draw
    /// has been drawn.
    fn settle(&mut self) {
        let until = Instant::now() + Duration::from_millis(300);
        while Instant::now() < until {
            self.drain();
            std::thread::sleep(Duration::from_millis(5));
        }
    }

    /// The one cell the cursor glyph stands in.
    fn cursor_cell(&self, what: &str) -> (u16, u16) {
        let glass = self.glass();
        let cells = glass.cursor_cells();
        assert_eq!(
            cells.len(),
            1,
            "{what}: fixture — the screen should show the cursor glyph in exactly one cell\n{}",
            (0..glass.h)
                .map(|y| glass.line(y))
                .collect::<Vec<_>>()
                .join("\n")
        );
        cells[0]
    }
}

/// A launch of the game outside the `Game` harness, for launches that never
/// draw a frame: its terminal's master side, where it has one, read while it
/// runs; killed and reaped on every path, by `Drop`.
struct Bare {
    child: Child,
    master: Option<File>,
    out: Vec<u8>,
}

impl Bare {
    /// Launch with `home` as its home, in a session of its own with no
    /// controlling terminal, its standard streams as given.
    fn spawn(
        home: &Path,
        stdin: Stdio,
        stdout: Stdio,
        stderr: Stdio,
        master: Option<File>,
    ) -> Bare {
        let mut command = Command::new(CLOWDER);
        command
            .env_clear()
            .env("HOME", home)
            .env("XDG_DATA_HOME", home.join(".local/share"))
            .env("PATH", "/usr/bin:/bin")
            .env("TERM", "xterm-256color")
            .env("COLORTERM", "truecolor")
            .current_dir(home)
            .stdin(stdin)
            .stdout(stdout)
            .stderr(stderr);
        // SAFETY: only `setsid`, which is async-signal-safe, runs between
        // fork and exec.
        unsafe {
            command.pre_exec(|| {
                if sys::setsid() < 0 {
                    return Err(io::Error::last_os_error());
                }
                Ok(())
            });
        }
        let child = command
            .spawn()
            .unwrap_or_else(|e| panic!("spawning {CLOWDER}: {e}"));
        drop(command);
        Bare {
            child,
            master,
            out: Vec::new(),
        }
    }

    /// Wait, reading the terminal all the while, for the launch to end.
    fn finish(&mut self, what: &str) -> ExitStatus {
        let deadline = Instant::now() + START_BUDGET;
        let mut buf = [0u8; 16 * 1024];
        loop {
            if let Some(master) = self.master.as_mut() {
                loop {
                    match master.read(&mut buf) {
                        Ok(0) => break,
                        Ok(n) => self.out.extend_from_slice(&buf[..n]),
                        Err(e) if e.kind() == io::ErrorKind::Interrupted => {}
                        Err(_) => break,
                    }
                }
            }
            if let Some(status) = self.child.try_wait().expect("polling the child") {
                return status;
            }
            assert!(
                Instant::now() < deadline,
                "{what}: the launch was still running {START_BUDGET:?} later"
            );
            std::thread::sleep(Duration::from_millis(5));
        }
    }
}

impl Drop for Bare {
    fn drop(&mut self) {
        let _ = self.child.kill();
        self.master.take();
        let _ = self.child.wait();
    }
}

/// Where the first text that is not part of a control sequence begins.
fn first_text(out: &[u8]) -> Option<usize> {
    let mut i = 0;
    while i < out.len() {
        if out[i] == 0x1b {
            if out.get(i + 1) == Some(&b'[') {
                let mut j = i + 2;
                while j < out.len() && !(0x40..=0x7e).contains(&out[j]) {
                    j += 1;
                }
                i = j + 1;
            } else {
                i += 2;
            }
            continue;
        }
        if out[i].is_ascii_alphanumeric() {
            return Some(i);
        }
        i += 1;
    }
    None
}

// ---------------------------------------------------------------------------
// TUI-RUN-060 — turned on at startup, exactly as the design asks
// ---------------------------------------------------------------------------

/// The game asks the terminal for normal tracking and the SGR encoding, each
/// once, with the rest of the terminal at startup — after entering the
/// alternate screen, before its first frame is finished, in play and while
/// waking — and sets or resets no other mouse mode: not X10 compatibility,
/// not button-motion or any-motion tracking, not the UTF-8, urxvt or
/// SGR-pixel encodings. The input source and its waits are kept: the quit key
/// still reaches the game, in play and while waking.
/// @spec TUI-RUN-060
#[test]
fn mouse_reporting_is_turned_on_at_startup_as_normal_tracking_in_the_sgr_encoding_and_nothing_else()
{
    for phase in [Phase::Play, Phase::Waking] {
        for terminal in [Terminal::Controlling, Terminal::NotControlling] {
            let what = format!("startup ({phase:?}, {terminal:?})");
            let mut game = Game::launch(
                "mouse-on",
                phase,
                terminal,
                Frames::ToTheTerminal,
                Parting::ToAFile,
            );
            expect_reporting_turned_on(&game.drawn, &what);
            game.press(b"q");
            game.expect_a_clean_ending(&what);
        }
    }
}

/// A launch that refuses to start — here because another session holds the
/// save — says so on an ordinary terminal and never turns reporting on.
/// @spec TUI-RUN-060
#[test]
fn a_launch_that_refuses_to_start_never_turns_mouse_reporting_on() {
    let mut holder = Game::launch(
        "mouse-refused",
        Phase::Play,
        Terminal::Controlling,
        Frames::ToTheTerminal,
        Parting::ToAFile,
    );
    let mut refused = {
        let _spawning = SPAWNING.lock().unwrap_or_else(|e| e.into_inner());
        let (master, term) = open_terminal();
        let refused = Bare::spawn(
            &holder.home.path,
            Stdio::from(term.try_clone().unwrap()),
            Stdio::from(term.try_clone().unwrap()),
            Stdio::from(term.try_clone().unwrap()),
            Some(master),
        );
        drop(term);
        refused
    };
    let status = refused.finish("a second launch on a held save");
    assert!(
        !status.success() && status.signal().is_none(),
        "fixture: a second launch on a save another session holds refuses to start ({status})"
    );
    let asked: Vec<ModeChange> = mode_changes(&refused.out)
        .into_iter()
        .filter(|c| c.on && (c.mode == NORMAL_TRACKING || c.mode == SGR_ENCODING))
        .collect();
    assert!(
        asked.is_empty(),
        "a launch that refused to start turned mouse reporting on: {asked:?}"
    );
    holder.press(b"q");
    holder.expect_a_clean_ending("the session holding the save");
}

// ---------------------------------------------------------------------------
// TUI-RUN-061 — turned off on every way out
// ---------------------------------------------------------------------------

/// Whatever ends the session — the quit key, a termination signal, the
/// terminal closed under it with or without a hangup — in play and while
/// waking, reporting is turned off on the way out: the SGR encoding, then
/// normal tracking, once each, before the alternate screen is left, and no
/// mode the game did not turn on is turned off. Where the terminal is still
/// there, it is cooked again once the game has gone.
/// @spec TUI-RUN-061, TUI-RUN-002
#[test]
fn mouse_reporting_is_turned_off_before_the_screen_is_handed_back_on_every_way_out() {
    for phase in [Phase::Play, Phase::Waking] {
        for ending in [Ending::QuitKey, Ending::Signal] {
            let what = format!("{ending:?} ({phase:?})");
            let mut game = Game::launch(
                "mouse-off",
                phase,
                Terminal::Controlling,
                Frames::ToTheTerminal,
                Parting::ToAFile,
            );
            let from = game.drawn.len();
            match ending {
                Ending::QuitKey => game.press(b"q"),
                Ending::Signal => game.terminate(),
                Ending::LostTerminal => unreachable!(),
            }
            game.expect_a_clean_ending(&what);
            expect_reporting_turned_on(&game.drawn, &what);
            expect_reporting_turned_off(&game.drawn, from, &what);
            assert_eq!(
                local_modes(game.master.as_ref().unwrap()),
                (true, true),
                "{what}: the terminal was left in raw mode"
            );
        }
        {
            let terminal = Terminal::Controlling;
            // Frames go to a pipe that outlives the terminal, so what the
            // shutdown writes on its way out can be read. Only a controlling
            // terminal: a closed one the game does not control, with frames
            // elsewhere, is not detected (Deferred 5 of the runtime LLD).
            let what = format!("the terminal closed ({phase:?}, {terminal:?})");
            let mut game = Game::launch(
                "mouse-off-lost",
                phase,
                terminal,
                Frames::ToAPipe,
                Parting::ToAFile,
            );
            let from = game.drawn.len();
            game.close_the_terminal();
            game.expect_a_clean_ending(&what);
            expect_reporting_turned_on(&game.drawn, &what);
            expect_reporting_turned_off(&game.drawn, from, &what);
        }
    }
}

// ---------------------------------------------------------------------------
// TUI-RUN-062 — entering fails partway
// ---------------------------------------------------------------------------

/// Raw mode itself cannot be entered — standard input is no terminal and the
/// launch has none to open — and entering has failed partway as surely as
/// at any later step: reporting, then the alternate screen, are turned off
/// before a word of the refusal is said, harmless to a terminal that never
/// turned them on, and reporting is never turned on.
/// @spec TUI-RUN-062
#[test]
fn where_raw_mode_cannot_be_entered_reporting_and_the_screen_are_turned_off_before_the_refusal() {
    // Raw mode fails: standard input is no terminal and the launch has none
    // to open. Its output and its words share one file, so their order can be
    // read.
    let home = TempDir::new("enter-raw-fails");
    let out_path = home.path.join("out.txt");
    let mut bare = {
        let _spawning = SPAWNING.lock().unwrap_or_else(|e| e.into_inner());
        let out = File::create(&out_path).expect("creating the output file");
        Bare::spawn(
            &home.path,
            Stdio::null(),
            Stdio::from(out.try_clone().unwrap()),
            Stdio::from(out),
            None,
        )
    };
    let what = "raw mode cannot be entered";
    let status = bare.finish(what);
    assert!(
        !status.success() && status.signal().is_none(),
        "{what}: the launch should refuse to start ({status})"
    );
    let out = fs::read(&out_path).expect("reading the output");
    let said = first_text(&out).unwrap_or_else(|| panic!("{what}: the failure was never said"));
    let changes = mode_changes(&out);
    let off_at = |mode: u32| {
        changes
            .iter()
            .find(|c| c.mode == mode && !c.on)
            .map(|c| c.at)
    };
    let (sgr_off, normal_off, left) = (
        off_at(SGR_ENCODING),
        off_at(NORMAL_TRACKING),
        off_at(ALTERNATE_SCREEN),
    );
    assert!(
        sgr_off.is_some() && normal_off.is_some() && left.is_some(),
        "{what}: entering failed partway, so reporting and the alternate screen are turned off \
         before the launch refuses, however far entering got (mode changes: {:?})",
        screen_and_mouse_modes(&out)
    );
    assert!(
        sgr_off < normal_off && normal_off < left && left < Some(said),
        "{what}: the SGR encoding, normal tracking, then the alternate screen are turned off, all \
         before the failure is said (mode changes: {:?}, failure said at {said})",
        screen_and_mouse_modes(&out)
    );
    let asked: Vec<&ModeChange> = changes
        .iter()
        .filter(|c| c.on && (c.mode == NORMAL_TRACKING || c.mode == SGR_ENCODING))
        .collect();
    assert!(
        asked.is_empty(),
        "{what}: reporting was turned on though raw mode never was: {asked:?}"
    );
}

/// The alternate screen cannot be written — the launch's output refuses
/// every write — after raw mode was entered on a real terminal: raw mode is
/// undone, the refusal is said, and the launch fails.
/// @spec TUI-RUN-062
#[test]
fn where_the_alternate_screen_cannot_be_written_raw_mode_is_undone_before_the_refusal() {
    let home = TempDir::new("enter-screen-fails");
    let (mut bare, master) = {
        let _spawning = SPAWNING.lock().unwrap_or_else(|e| e.into_inner());
        let (master, term) = open_terminal();
        let (reader, writer) = io::pipe().expect("a pipe");
        drop(reader);
        let bare = Bare::spawn(
            &home.path,
            Stdio::from(term.try_clone().unwrap()),
            Stdio::from(writer),
            Stdio::from(File::create(home.path.join("stderr.txt")).unwrap()),
            None,
        );
        drop(term);
        (bare, master)
    };
    let what = "the alternate screen cannot be written";
    let status = bare.finish(what);
    assert!(
        !status.success() && status.signal().is_none(),
        "{what}: the launch should refuse to start ({status})"
    );
    assert!(
        !fs::read_to_string(home.path.join("stderr.txt"))
            .unwrap_or_default()
            .trim()
            .is_empty(),
        "{what}: the failure was never said"
    );
    assert_eq!(
        local_modes(&master),
        (true, true),
        "{what}: the terminal was left in raw mode"
    );
}

// ---------------------------------------------------------------------------
// TUI-RUN-063 — on for the whole session
// ---------------------------------------------------------------------------

/// Every key a terminal sends without a modifier chord, except the two that
/// quit: the printable characters, the control characters but Ctrl-C and
/// Esc, Tab and Shift-Tab, the arrows and the editing keys, and the function
/// keys.
fn every_key_but_quitting() -> Vec<Vec<u8>> {
    let mut keys: Vec<Vec<u8>> = (0x20u8..0x7f)
        .filter(|b| *b != b'q')
        .map(|b| vec![b])
        .collect();
    keys.extend(
        (0x01u8..0x20)
            .filter(|b| *b != 0x03 && *b != 0x1b)
            .map(|b| vec![b]),
    );
    keys.push(vec![0x7f]);
    for seq in [
        "\x1b[A", "\x1b[B", "\x1b[C", "\x1b[D", "\x1b[Z", "\x1b[H", "\x1b[F", "\x1b[2~", "\x1b[3~",
        "\x1b[5~", "\x1b[6~", "\x1bOP", "\x1bOQ", "\x1bOR", "\x1bOS", "\x1b[15~", "\x1b[17~",
        "\x1b[18~", "\x1b[19~", "\x1b[20~", "\x1b[21~", "\x1b[23~", "\x1b[24~",
    ] {
        keys.push(seq.as_bytes().to_vec());
    }
    keys
}

/// Nothing the player presses turns reporting off, in play and while waking:
/// with every key but the two that quit pressed, each followed by Esc, the
/// game has turned reporting on once and off never — until it is ended, when
/// it is turned off once.
/// @spec TUI-RUN-063
#[test]
fn no_key_turns_mouse_reporting_off_while_the_game_runs() {
    for phase in [Phase::Play, Phase::Waking] {
        let what = format!("every key pressed ({phase:?})");
        let mut game = Game::launch(
            "mouse-keys",
            phase,
            Terminal::Controlling,
            Frames::ToTheTerminal,
            Parting::ToAFile,
        );
        for key in every_key_but_quitting() {
            game.send(&key);
            std::thread::sleep(Duration::from_millis(15));
            // Alone, so it is read as Esc and not as the start of a chord.
            game.send(b"\x1b");
            std::thread::sleep(Duration::from_millis(40));
            game.drain();
            assert!(
                game.child.try_wait().expect("polling the child").is_none(),
                "{what}: fixture — the game ended on {key:?}, which is not a quit key"
            );
        }
        game.settle();
        let from = game.drawn.len();
        game.terminate();
        game.expect_a_clean_ending(&what);
        expect_reporting_turned_on(&game.drawn, &what);
        expect_reporting_turned_off(&game.drawn, from, &what);
    }
}

/// Every file beneath `dir`, as paths relative to it.
fn files_beneath(dir: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let mut stack = vec![dir.to_path_buf()];
    while let Some(d) = stack.pop() {
        for entry in fs::read_dir(&d).into_iter().flatten().flatten() {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else {
                out.push(path.strip_prefix(dir).unwrap().to_path_buf());
            }
        }
    }
    out.sort();
    out
}

/// No launch option turns reporting off — the game launched with any of the
/// options one might reach for still turns it on — and the game stores no
/// setting that could: a session leaves nothing in its home but its save.
/// @spec TUI-RUN-063
#[test]
fn no_launch_option_turns_mouse_reporting_off_and_no_setting_is_stored() {
    let options: [&[&str]; 8] = [
        &["--force"],
        &["--no-mouse"],
        &["--nomouse"],
        &["--mouse=off"],
        &["--mouse", "off"],
        &["--no-mouse-capture"],
        &["--keyboard-only"],
        &["-m"],
    ];
    for args in options {
        let what = format!("launched with {args:?}");
        let mut game = Game::launch_with(
            "mouse-option",
            Phase::Play,
            Terminal::Controlling,
            Frames::ToTheTerminal,
            Parting::ToAFile,
            args,
        );
        expect_reporting_turned_on(&game.drawn, &what);
        game.press(b"q");
        game.expect_a_clean_ending(&what);
        let save = game
            .save_path()
            .strip_prefix(&game.home.path)
            .unwrap()
            .to_path_buf();
        let stored: Vec<PathBuf> = files_beneath(&game.home.path)
            .into_iter()
            .filter(|p| *p != save && *p != Path::new("stderr.txt"))
            .collect();
        assert!(
            stored.is_empty(),
            "{what}: the session left files beside its save: {stored:?}"
        );
    }
}

// ---------------------------------------------------------------------------
// TUI-RUN-065 — pointer events reach navigation
// ---------------------------------------------------------------------------

/// A wheel step reported in the SGR encoding over the list moves the cursor
/// down a row and back, and a left click on a row's line puts the cursor on
/// the row under it — on the real terminal, through the input source the game
/// is built with.
/// @spec TUI-RUN-065, TUI-RUN-060
#[test]
fn a_click_sent_as_an_sgr_report_moves_the_cursor_to_the_row_under_it() {
    let mut game = Game::launch(
        "mouse-click",
        Phase::Lively,
        Terminal::Controlling,
        Frames::ToTheTerminal,
        Parting::ToAFile,
    );
    let (x0, y0) = game.cursor_cell("the opening");

    game.send(&wheel_down_at(x0 + 4, y0));
    let moved = game.wait_for(
        EXIT_BUDGET,
        |g| matches!(g.cursor_cells()[..], [(x, y)] if x == x0 && y > y0),
    );
    assert!(
        moved,
        "a wheel step down over the list did not move the cursor down a row"
    );
    let (_, y1) = game.cursor_cell("after a wheel step down");

    game.send(&wheel_up_at(x0 + 4, y1));
    assert!(
        game.wait_for(EXIT_BUDGET, |g| g.cursor_cells() == [(x0, y0)]),
        "a wheel step up over the list did not move the cursor back up a row"
    );

    game.send(&left_click(x0 + 6, y1));
    assert!(
        game.wait_for(EXIT_BUDGET, |g| g.cursor_cells() == [(x0, y1)]),
        "a left click on the line of the row below the cursor did not put the cursor on it"
    );
    game.send(&left_click(x0 + 2, y0));
    assert!(
        game.wait_for(EXIT_BUDGET, |g| g.cursor_cells() == [(x0, y0)]),
        "a left click on the line of the row above the cursor did not put the cursor on it"
    );

    let from = game.drawn.len();
    game.press(b"q");
    game.expect_a_clean_ending("a click");
    expect_reporting_turned_off(&game.drawn, from, "a click");
}

// ---------------------------------------------------------------------------
// TUI-RUN-064 — a burst of pointer events does not hold back a key
// ---------------------------------------------------------------------------

/// A burst of wheel reports and the quit key behind it, sent at once: forty
/// steps down the list and forty back up, more than a frame's bound on keys.
/// The key is taken in the frame that takes the reports — so no frame is
/// drawn with the cursor anywhere but where it started, which a frame taking
/// only part of the burst would draw — and every step is applied before it,
/// so the game quits with the cursor where it began.
/// @spec TUI-RUN-064
#[test]
fn a_burst_of_wheel_reports_does_not_hold_back_the_key_behind_it() {
    let mut game = Game::launch(
        "mouse-burst",
        Phase::Lively,
        Terminal::Controlling,
        Frames::ToTheTerminal,
        Parting::ToAFile,
    );
    let (x0, y0) = game.cursor_cell("the opening");
    game.send(&wheel_down_at(x0 + 4, y0));
    assert!(
        game.wait_for(
            EXIT_BUDGET,
            |g| matches!(g.cursor_cells()[..], [(x, y)] if x == x0 && y > y0)
        ),
        "fixture: a wheel step down over the list moves the cursor"
    );
    let (_, y1) = game.cursor_cell("after a wheel step down");
    game.send(&left_click(x0 + 4, y0));
    assert!(
        game.wait_for(EXIT_BUDGET, |g| g.cursor_cells() == [(x0, y0)]),
        "fixture: a click puts the cursor back on the first row"
    );
    assert!(y1 > y0);
    game.settle();

    let from = game.drawn.len();
    let mut burst = Vec::new();
    for _ in 0..40 {
        burst.extend(wheel_down_at(x0 + 4, y0));
    }
    for _ in 0..40 {
        burst.extend(wheel_up_at(x0 + 4, y0));
    }
    burst.push(b'q');
    game.send(&burst);
    game.expect_a_clean_ending("a burst of wheel reports, then the quit key");

    let drawn_elsewhere: Vec<(usize, u16, u16)> = game
        .glass()
        .glyphs
        .into_iter()
        .filter(|(at, x, y)| *at >= from && (*x, *y) != (x0, y0))
        .collect();
    assert!(
        drawn_elsewhere.is_empty(),
        "the burst was split across frames: a frame was drawn with the cursor part-way down the \
         list ({} cursor draws away from the first row), so the key behind the burst waited for a \
         later frame",
        drawn_elsewhere.len()
    );
}

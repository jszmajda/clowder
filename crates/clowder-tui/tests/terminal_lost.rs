//! Losing the terminal — the built game on a real pseudo-terminal, with the
//! terminal taken away.
//!
//! @spec TUI-RUN-046, TUI-RUN-047, TUI-RUN-048, TUI-RUN-050, TUI-RUN-051,
//! TUI-RUN-052, TUI-RUN-057, TUI-RUN-058, TUI-RUN-059
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
        assert!(sys::fcntl(fd, sys::F_SETFD, sys::FD_CLOEXEC) >= 0, "F_SETFD: {}", io::Error::last_os_error());
        assert_eq!(sys::grantpt(fd), 0, "grantpt: {}", io::Error::last_os_error());
        assert_eq!(sys::unlockpt(fd), 0, "unlockpt: {}", io::Error::last_os_error());
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
    let size = sys::Winsize { rows: 30, cols: 100, xpixel: 0, ypixel: 0 };
    // SAFETY: TIOCSWINSZ reads one `winsize` from the pointer it is given.
    let set = unsafe { sys::ioctl(terminal.as_raw_fd(), sys::TIOCSWINSZ, &size as *const sys::Winsize) };
    assert_eq!(set, 0, "TIOCSWINSZ: {}", io::Error::last_os_error());
    make_nonblocking(master.as_raw_fd());
    (master, terminal)
}

/// The terminal's local modes — whether it is cooked (canonical, echoing) or
/// raw — asked of its master side, which goes on answering after the game
/// has gone (the terminal side of a session's controlling terminal is revoked
/// when the session's leader exits).
fn local_modes(master: &File) -> (bool, bool) {
    let mut t = sys::Termios { c_iflag: 0, c_oflag: 0, c_cflag: 0, c_lflag: 0, rest: [0; 256] };
    // SAFETY: `Termios` is at least as large as the platform's `struct termios`.
    let got = unsafe { sys::tcgetattr(master.as_raw_fd(), &mut t) };
    assert_eq!(got, 0, "tcgetattr: {}", io::Error::last_os_error());
    (t.c_lflag & sys::ICANON != 0, t.c_lflag & sys::ECHO != 0)
}

// ---------------------------------------------------------------------------
// Scratch homes and fixture saves
// ---------------------------------------------------------------------------

/// A scratch home that cleans itself up, named uniquely per call: tests run
/// in parallel in one process.
struct TempDir {
    path: PathBuf,
}

impl TempDir {
    fn new(tag: &str) -> TempDir {
        static COUNTER: AtomicU32 = AtomicU32::new(0);
        let n = COUNTER.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!("clowder-lost-{}-{}-{}", std::process::id(), tag, n));
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
    clowder_core::begin_load(save_contents, 0).expect("the save loads").total_ticks()
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
    fn launch(tag: &str, phase: Phase, terminal: Terminal, frames: Frames, parting: Parting) -> Game {
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
            .stdin(Stdio::from(term.try_clone().expect("duplicating the terminal")));
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
        let mut child = command.spawn().unwrap_or_else(|e| panic!("spawning {CLOWDER}: {e}"));
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
            .map(|rd| rd.flatten().map(|e| e.file_name().to_string_lossy().into_owned()).collect())
            .unwrap_or_default();
        names.retain(|n| Some(n.as_str()) != save.file_name().and_then(|s| s.to_str()));
        names.sort();
        names
    }

    fn locks(&self) -> Vec<String> {
        self.beside_the_save().into_iter().filter(|n| n.contains("lock")).collect()
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
            let framed = find(&self.drawn, ENTER).is_some_and(|at| find(&self.drawn[at..], HIDE).is_some());
            if framed {
                break;
            }
            if let Some(status) = self.child.try_wait().expect("polling the child") {
                panic!(
                    "the game ended ({status}) before it drew a frame.\n--- stderr ---\n{}",
                    self.stderr()
                );
            }
            assert!(Instant::now() < deadline, "the game drew no frame within {START_BUDGET:?} of starting");
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
        self.master.as_mut().expect("the terminal is still open").write_all(keys).expect("typing at the game");
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
        assert!(!self.stderr().contains("panicked at"), "{what}: the game panicked on its way out.\n{}", self.stderr());
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
            Phase::Play => {}
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
                let whole = clowder_core::begin_load(fixture, now).expect("the fixture loads").total_ticks();
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
    let asked = [&crate_manifest, &workspace_manifest].into_iter().any(|path| {
        let text = fs::read_to_string(path).unwrap_or_else(|e| panic!("reading {}: {e}", path.display()));
        text.lines().any(|line| {
            let line = line.trim_start();
            line.starts_with("crossterm") && !line.starts_with('#') && line.contains("\"use-dev-tty\"")
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
        let mut game = Game::launch("close-play", Phase::Play, terminal, Frames::ToTheTerminal, Parting::ToTheTerminal);
        assert!(!game.save_path().exists(), "fixture: a new game has no save until it shuts down");
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
            let order = if signal_first { "signal, then terminal closed" } else { "terminal closed, then signal" };
            let what = format!("{order}, in play ({terminal:?})");
            let mut game =
                Game::launch("close-term", Phase::Play, terminal, Frames::ToTheTerminal, Parting::ToTheTerminal);
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
        let mut game = Game::launch("quit-play", Phase::Play, terminal, Frames::ToTheTerminal, Parting::ToAFile);
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
        let mut game =
            Game::launch("close-waking", Phase::Waking, terminal, Frames::ToTheTerminal, Parting::ToTheTerminal);
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
        let order = if signal_first { "signal, then terminal closed" } else { "terminal closed, then signal" };
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
        let mut game = Game::launch("quit-waking", Phase::Waking, terminal, Frames::ToTheTerminal, Parting::ToAFile);
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
        let mut game = Game::launch("term", phase, Terminal::Controlling, Frames::ToTheTerminal, Parting::ToAFile);
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
        let mut game = Game::launch("refused", phase, Terminal::Controlling, Frames::ToAPipe, Parting::ToAFile);
        let raw = local_modes(game.master.as_ref().unwrap());
        assert_eq!(raw, (false, false), "fixture: a running game holds its terminal in raw mode");

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
        let mut game = Game::launch("restored-once", phase, Terminal::Controlling, Frames::ToAPipe, Parting::ToAFile);
        let left_before = game.drawn.windows(LEAVE.len()).filter(|w| *w == LEAVE).count();
        assert_eq!(left_before, 0, "fixture: a running game has not left the alternate screen");

        game.close_the_terminal();
        game.expect_a_clean_ending(&what);

        let left = game.drawn.windows(LEAVE.len()).filter(|w| *w == LEAVE).count();
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
            let mut game =
                Game::launch("unwritable", phase, Terminal::NotControlling, Frames::ToTheTerminal, Parting::ToAFile);
            game.make_the_save_unwritable();
            match ending {
                Ending::QuitKey => game.press(b"q"),
                Ending::Signal => game.terminate(),
                Ending::LostTerminal => game.close_the_terminal(),
            }
            game.expect_a_quits_exit(&what);
            game.expect_no_lock(&what);

            let stderr = game.stderr();
            let named: Vec<&str> = stderr.lines().filter(|l| l.to_lowercase().contains("save")).collect();
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
            let mut game =
                Game::launch("unsaid", phase, terminal, Frames::ToTheTerminal, Parting::ToTheTerminal);
            game.make_the_save_unwritable();
            game.close_the_terminal();
            game.expect_a_quits_exit(&what);
            game.expect_no_lock(&what);
        }
    }
}

use fs2::{lock_contended_error, FileExt};
use gtk::{
    glib::{self, GString},
    prelude::*,
    Application, ApplicationWindow, TextBuffer, TextView, WrapMode,
};
use std::{
    env,
    ffi::OsStr,
    fmt,
    fs::File,
    io::{self, Read, Seek, SeekFrom, Write},
    path::PathBuf,
    str,
};

const APP_ID: &str = "org.github.z_snails.gluey";
const WELCOME_TEXT: &str = "Welcome to Gluey";

#[derive(Debug)]
struct Config {
    text: GString,
    config_file: File,
}

#[derive(Debug)]
struct AlreadyOpen;

impl fmt::Display for AlreadyOpen {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "An instance of gluey is already running")
    }
}

impl std::error::Error for AlreadyOpen {}

impl Config {
    fn new(config_file: File) -> Self {
        Self {
            text: WELCOME_TEXT.into(),
            config_file,
        }
    }

    fn serialize(&self, mut w: impl Write) -> io::Result<()> {
        write!(w, "{}", self.text)
    }

    fn deserialize(data: &[u8], file: File) -> anyhow::Result<Self> {
        let text = GString::from(str::from_utf8(data)?.to_owned());
        Ok(Config {
            text,
            config_file: file,
        })
    }

    fn load() -> anyhow::Result<Self> {
        let config_loc = get_config_loc();
        eprintln!("config file location: {config_loc:?}");
        if let Ok(mut config_file) = File::options().read(true).write(true).open(&config_loc) {
            match config_file.try_lock_exclusive() {
                Ok(()) => {}
                Err(err) if err.kind() == lock_contended_error().kind() => Err(AlreadyOpen)?,
                Err(err) => Err(err)?,
            };
            let mut config = Vec::new();
            config_file.read_to_end(&mut config)?;
            Ok(Config::deserialize(&config, config_file)?)
        } else {
            let config_file = File::options()
                .write(true)
                .create(true)
                .truncate(true)
                .open(&config_loc)?;
            config_file.try_lock_exclusive()?;
            Ok(Config::new(config_file))
        }
    }

    fn save(mut self) -> io::Result<()> {
        self.config_file.seek(SeekFrom::Start(0))?;
        self.config_file.set_len(0)?;
        self.serialize(&self.config_file)
    }
}

fn get_config_loc() -> PathBuf {
    let config_loc = if let Some(config) = env::var_os("GLUEY_CONFIG") {
        PathBuf::from(config)
    } else if let Some(home) = env::var_os("XDG_CONFIG_HOME").or_else(|| env::var_os("HOME")) {
        [&home, OsStr::new(".gluey/config")].iter().collect()
    } else {
        panic!("GLUEY_CONFIG, XDG_CONFIG_HOME and HOME not set");
    };
    config_loc
}

fn build_success(app: &Application, text_buffer: &TextBuffer) {
    let text = TextView::builder()
        .buffer(text_buffer)
        .wrap_mode(WrapMode::WordChar)
        .build();

    let window = ApplicationWindow::builder()
        .application(app)
        .title("Gluey")
        .child(&text)
        .build();

    window.present()
}

fn main() -> anyhow::Result<()> {
    let mut config = Config::load()?;
    eprintln!("config on open: {config:?}");

    let text_buffer = TextBuffer::builder().text(&config.text).build();

    let app = Application::builder().application_id(APP_ID).build();
    app.connect_activate(
        glib::clone!(@strong text_buffer => move |app| build_success(app, &text_buffer)),
    );
    app.run();

    let (start, end) = text_buffer.bounds();
    config.text = text_buffer.slice(&start, &end, true);
    eprintln!("config on close: {config:?}");
    config.save()?;

    Ok(())
}

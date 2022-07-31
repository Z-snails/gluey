use fs2::FileExt;
use gtk::{
    glib::{self, GString},
    prelude::*,
    Application, ApplicationWindow, TextBuffer, TextView, WrapMode,
};
use std::{
    env,
    ffi::OsStr,
    fs,
    io::{self, Read, Write},
    path::PathBuf,
    str,
};

const APP_ID: &str = "org.github.z_snails.gluey";
const WELCOME_TEXT: &str = "Welcome to Gluey";

#[derive(Debug)]
pub struct Config {
    text: GString,
}

impl Config {
    fn new() -> Self {
        Self {
            text: WELCOME_TEXT.into(),
        }
    }

    fn serialize(&self, mut w: impl Write) -> io::Result<()> {
        write!(w, "{}", self.text)
    }

    fn deserialize(data: &[u8]) -> anyhow::Result<Self> {
        let text = GString::from(str::from_utf8(data)?.to_owned());
        Ok(Config { text })
    }

    fn load() -> anyhow::Result<Self> {
        let config_loc = get_config_loc();
        eprintln!("config file location: {config_loc:?}");
        Ok(if let Ok(mut config_file) = fs::File::open(config_loc) {
            config_file.lock_shared()?;
            let mut config = Vec::new();
            config_file.read_to_end(&mut config)?;
            Config::deserialize(&config)?
        } else {
            Config::new()
        })
    }

    fn save(&self) -> io::Result<()> {
        let config_loc = get_config_loc();
        config_loc
            .parent()
            .map(|config_dir| fs::DirBuilder::new().recursive(true).create(config_dir));
        let mut config_file = fs::File::create(&config_loc)?;
        config_file.lock_exclusive()?;
        self.serialize(&mut config_file)
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

fn build_ui(app: &Application, text_buffer: &TextBuffer) {
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

    let app = Application::builder().application_id(APP_ID).build();
    let text_buffer = TextBuffer::builder().text(&config.text).build();
    app.connect_activate(
        glib::clone!(@strong text_buffer => move |app| build_ui(app, &text_buffer)),
    );
    app.run();

    let (start, end) = text_buffer.bounds();
    config.text = text_buffer.slice(&start, &end, true);
    eprintln!("config on close: {config:?}");
    config.save()?;

    Ok(())
}

use log::{Metadata, Record};

pub(crate) struct SimpleLogger(());
const LOGGER: &SimpleLogger = &SimpleLogger(());

impl SimpleLogger {
    pub(crate) fn init() -> Result<(), log::SetLoggerError> {
        log::set_logger(LOGGER)
    }
}

impl log::Log for SimpleLogger {
    fn enabled(&self, _metadata: &Metadata) -> bool {
        true
    }

    fn log(&self, record: &Record) {
        if self.enabled(record.metadata()) {
            eprintln!(
                "{} {}: {}",
                record.level(),
                record.target(),
                record.args()
            );
        }
    }

    fn flush(&self) {}
}

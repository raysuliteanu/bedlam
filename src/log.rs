use log::{LevelFilter, info};
use log4rs::append::file::FileAppender;
use log4rs::config::{Appender, Config, Logger, Root};
use log4rs::encode::pattern::PatternEncoder;

pub fn init() -> anyhow::Result<()> {
    let file_appender = FileAppender::builder()
        .encoder(Box::new(PatternEncoder::new("{d} - {m}{n}")))
        .build("bedlam.log")
        .unwrap();

    let config = Config::builder()
        .appender(Appender::builder().build("bedlam", Box::new(file_appender)))
        .logger(
            Logger::builder()
                .appender("bedlam")
                .additive(false)
                .build("bedlam", LevelFilter::Debug),
        )
        .build(Root::builder().appender("bedlam").build(LevelFilter::Debug))
        .unwrap();

    log4rs::init_config(config)?;

    info!("initialized logging");

    Ok(())
}

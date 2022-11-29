//extern crate serde_json;
use clap::{crate_authors, crate_name, crate_version, Arg};
use hyper::{Body, Request};
use log::{error, info, trace, warn};
use std::env;
mod options;
use options::Options;
use prometheus_exporter_base::{render_prometheus, PrometheusInstance, PrometheusMetric};
use std::net::IpAddr;
use std::process::Command;
mod rule_engine;
pub use rule_engine::*;
use std::sync::Arc;
mod lsblk;
pub use lsblk::*;
use serde_json::Value;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ExporterError {
    #[error("device `{0}` is not supported at the moment")]
    UnsupportedDevice(String),
    #[error("device `{0}` has an unsupported SMART attributes table")]
    UnsupportedSMARTAttributeTable(String),
    #[error("device `{0}` has an unsupported root temperature attribute")]
    UnsupportedRootAttribute(String),
}

async fn perform_request(
    _req: Request<Body>,
    options: Arc<Options>,
) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    let block_devices = get_block_devices()?;
    //println!("{:#?}", block_devices);
    let mut pm = PrometheusMetric::build()
        .with_name("smartctl_device_temperature")
        .with_help("device temperature as reported by smartctl")
        .with_metric_type(prometheus_exporter_base::MetricType::Gauge)
        .build();

    for block_device in block_devices
        .into_iter()
        .filter(|device| device.block_device_type == "disk")
        .filter(|device| {
            !options
                .exclude_regexes
                .iter()
                .any(|regex| regex.is_match(&device.name))
        })
    {
        let temperature = match process_device(&block_device, &options) {
            Ok(temperature) => temperature,
            Err(error) => {
                warn!(
                    "Cannot process {}, skipping: {}",
                    block_device.signature(),
                    error
                );
                continue;
            }
        };

        pm.render_and_append_instance(
            &PrometheusInstance::new()
                .with_label("device", &block_device.signature().to_string() as &str)
                .with_value(temperature),
        );

        trace!("temperature {}", temperature);
    }

    Ok(pm.render())
}

fn process_device(
    block_device: &BlockDevice,
    options: &Arc<Options>,
) -> Result<i64, Box<dyn std::error::Error + Send + Sync>> {
    let output = if options.prepend_sudo {
        Command::new("sudo")
            .arg("smartctl")
            .arg("-n").arg("standby")
            .arg("-a")
            .arg("-j")
            .arg(format!("/dev/{}", block_device.name))
            .output()?
    } else {
        Command::new("smartctl")
            .arg("-n").arg("standby")
            .arg("-a")
            .arg("-j")
            .arg(format!("/dev/{}", block_device.name))
            .output()?
    };
    let output_stdout_str = String::from_utf8(output.stdout)?;
    trace!("output_stdout_str == {}", output_stdout_str,);
    let output_stderr_str = String::from_utf8(output.stderr)?;
    trace!("output_stderr_str == {}", output_stderr_str,);

    let output_json: Value = serde_json::from_str(&output_stdout_str)?;

    match process_rules(block_device, &output_json) {
        Some(temperature) => Ok(temperature),
        None => Err(Box::new(ExporterError::UnsupportedDevice(
            block_device.signature(),
        ))),
    }
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let matches = clap::App::new(crate_name!())
        .version(crate_version!())
        .author(crate_authors!("\n"))
        .arg(
            Arg::with_name("addr")
                .short("l")
                .help("exporter address")
                .default_value("0.0.0.0")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("port")
                .short("p")
                .help("exporter port")
                .default_value("9587")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("verbose")
                .short("v")
                .help("verbose logging")
                .takes_value(false),
        )
        .arg(
            Arg::with_name("prepend_sudo")
                .short("a")
                .help("Prepend sudo to the smartctl commands")
                .takes_value(false),
        )
        .arg(
            Arg::with_name("exclude_regexes")
                .short("e")
                .help("Exclude devices (ie /dev/sda) regexes")
                .multiple(true)
                .takes_value(true),
        )
        .get_matches();

    let options = Options::from_claps(&matches)?;

    if options.verbose {
        env::set_var(
            "RUST_LOG",
            format!("{}=trace,prometheus_exporter_base=trace", crate_name!()),
        );
    } else {
        env::set_var(
            "RUST_LOG",
            format!("{}=info,prometheus_exporter_base=info", crate_name!()),
        );
    }
    env_logger::init();

    info!(
        "{} v{} starting...",
        env!("CARGO_PKG_NAME"),
        env!("CARGO_PKG_VERSION")
    );
    info!("using options: {:?}", options);

    let bind = matches.value_of("port").unwrap();
    let bind = bind.parse::<u16>().expect("port must be a valid number");
    let ip = matches.value_of("addr").unwrap().parse::<IpAddr>().unwrap();
    let addr = (ip, bind).into();

    info!("starting exporter on http://{}/metrics", addr);

    render_prometheus(addr, options, |request, options| {
        Box::pin(perform_request(request, options))
    })
    .await;

    Ok(())
}

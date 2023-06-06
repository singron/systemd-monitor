extern crate dbus;
extern crate gethostname;
extern crate reqwest;
extern crate serde;
extern crate serde_json;
extern crate url;

use serde::Deserialize;
use std::fmt::Write;
use std::time::{Duration, Instant};

pub mod systemd_manager {
    #![allow(unused)]
    include!(concat!(env!("OUT_DIR"), "/systemd_manager.rs"));
}

pub mod systemd_service {
    #![allow(unused)]
    include!(concat!(env!("OUT_DIR"), "/systemd_service.rs"));
}

fn check_service<M: systemd_manager::OrgFreedesktopSystemd1Manager>(
    conn: &dbus::blocking::Connection,
    manager: &M,
    service_name: &str,
    error_msg: &mut String,
) -> Result<(), Box<dyn std::error::Error>> {
    let service_path = match manager.get_unit(service_name) {
        Ok(p) => p,
        Err(e) => {
            if e.name() == Some("org.freedesktop.systemd1.NoSuchUnit") {
                write!(error_msg, "{} not loaded", service_name).unwrap();
                return Ok(());
            } else {
                return Err(e.into());
            }
        }
    };
    let service = conn.with_proxy(
        "org.freedesktop.systemd1",
        service_path,
        std::time::Duration::from_millis(5000),
    );
    {
        use systemd_service::OrgFreedesktopSystemd1Service;
        use systemd_service::OrgFreedesktopSystemd1Unit;
        let active_state = service.active_state()?;
        let sub_state = service.sub_state()?;
        let result = service.result()?;

        let failed = active_state != "active" || result != "success";
        if failed {
            if !error_msg.is_empty() {
                write!(error_msg, ",").unwrap();
            }
            write!(
                error_msg,
                "{} active_state={} sub_state={}, result={}",
                service_name, active_state, sub_state, result
            )
            .unwrap();
        }
    }
    Ok(())
}

#[derive(Deserialize)]
struct Response {
    error: Option<String>,
}

fn send_status(
    client: &reqwest::blocking::Client,
    monitor_url: &str,
    status: &str,
    hostname: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let params = [("status", status), ("hostname", hostname)];
    let deadline = Instant::now() + Duration::from_secs(30);
    let mut tries = 0;
    while Instant::now() < deadline {
        if tries > 0 {
            std::thread::sleep(std::time::Duration::from_secs(tries));
        }
        tries += 1;
        let target_url = url::Url::parse_with_params(monitor_url, &params)?;
        let mut res: reqwest::blocking::Response = match client.get(target_url).send() {
            Ok(res) => res,
            Err(e) => {
                eprintln!("Error sending request: {}", e);
                continue;
            }
        };
        let res_status = res.status();
        use std::io::Read;
        let mut body = Vec::new();
        if let Err(e) = res.read_to_end(&mut body) {
            eprintln!("Error reading body: {}", e);
            continue;
        }
        if !res_status.is_success() {
            eprintln!("Error http code: {}", res_status);
            continue;
        }
        let rbody: Response = match serde_json::from_slice(&body) {
            Ok(b) => b,
            Err(e) => {
                eprintln!("Error parsing body: {}", e);
                continue;
            }
        };
        if let Some(e) = &rbody.error {
            return Err(format!("Error from server: {}", e).into());
        }
        return Ok(());
    }
    return Err(format!("deadline exceeded").into());
}

#[derive(Deserialize)]
struct Config {
    /// List of systemd services to monitor (e.g. "dbus.service").
    services: Vec<String>,
    /// A URL where a GET request will be sent with `hostname` and `status` parameters.
    monitor_url: String,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args();
    args.next();
    let config_path = args.next().expect("Argument should be path to config");
    let config: Config = serde_json::from_slice(&std::fs::read(config_path)?)?;
    if config.services.is_empty() {
        return Err("Error in config: services should not be empty".into());
    }
    if let Err(e) = url::Url::parse(&config.monitor_url) {
        return Err(format!("Error in config: monitor_url: {}", e).into());
    }
    let conn = dbus::blocking::Connection::new_system()?;
    let manager = conn.with_proxy(
        "org.freedesktop.systemd1",
        "/org/freedesktop/systemd1",
        std::time::Duration::from_millis(5000),
    );
    let mut error_message = String::new();
    for s in &config.services {
        check_service(&conn, &manager, s, &mut error_message)?;
    }
    let client = reqwest::blocking::Client::new();
    let hostname = gethostname::gethostname().into_string().unwrap();
    let status = if error_message.is_empty() {
        "ok"
    } else {
        eprintln!("Sending status: {}", &error_message);
        &error_message
    };
    send_status(&client, &config.monitor_url, status, &hostname)?;
    Ok(())
}

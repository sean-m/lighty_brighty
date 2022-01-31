/* 
    lighty_brighty
    All the trappings of a modern laptop OS are here in Gnome and Plasma, why
    is automatic screen brightness adjustment only found in Gnome? Well DBus
    exists and those Gnome folks really love it. Ambient light sensor data that
    gnome-shell uses for automatic screen brightness is exposed by the wonderful
    iio-sensor-proxy service.

    Some simple code and Plasma's dbus interface for screen brightness make it
    possible to have a screen brightness daemon that works without root privileges
    and only adjusts brightness when needed. This is not that service, this is
    a tribute.
*/

use std::cmp;
use regex::Regex;
use log::{info, error, debug};
use clap::{Arg, App};

use simplelog::*;

use crate::kde_powermanagement_suspendsession::SuspendSessionProxy;
use crate::kde_brightness_control_trait::BrightnessControlProxy;
use crate::sensor_proxy_trait::SensorProxyProxy;

use zbus::{Connection, Result};
use futures_util::stream::StreamExt;


mod sensor_proxy_trait;
mod kde_brightness_control_trait;
mod kde_powermanagement_suspendsession; 

fn get_light_step(lux_value : f64, step_count : f64) -> f64 {
    let lux = lux_value;
    let log_step = lux.log10() / 5.0;
    (step_count * log_step).max(1.0)
}

// Although we use `async-std` here, you can use any async runtime of choice.
#[async_std::main]
async fn main() -> Result<()> {
    //
    // Parse command arguments and configure settings
    //
    let matches = App::new("Lighty Brighty")
        .version("0.2.0")
        .author("Sean McArdle <sean@mcardletech.com>")
        .about("Changes the backlight brightness for KDE Plasma based on ambient light values (lux only for now).")
        .arg(
            Arg::new("skew")
            .short('s')
            .long("skew")
            .takes_value(true)
            .help("Float value that skews the backlight setting. 0.8 (80%) works best for me.")
            .default_value("0.8")
            )
        .arg(
            Arg::new("log")
            .short('l')
            .long("log")
            .takes_value(true)
            .help("Log level. Can be one of the following: debug, info, warn (default), error.")
            .default_value("warn")
            )
        .get_matches();


    // Threshold that needs to be crossed in lux values before adjusting screen brightness. Hopefully prevents herky jerky changes. 
    // TODO make this an argument
    // TODO have multiple levels of sensitivity as thresholds for setting brightness autmatically vs. honoring the user setting.
    let sensitivity = 10_usize;
    
    // Multiplied to the brightness setting before telling Plasma to change.
    let mut skew = 0.8_f64;
    let skew_str = matches.value_of("skew");
    match skew_str {
        Some(s) => {
            match s.parse::<f64>() {
                Ok(n) => { skew = n; }
                Err(_e) => { 
                   error!("Skew value must be between 0.0 and 1.0. Got: {}", s);
                   return Ok(())
                }
            }
        }
        None => { assert!(false, "If you got here the default skew value parsed by clap didn't work!"); } 
    }

    // Set log level for more verbose output.
    let mut log_level : LevelFilter = LevelFilter::Warn;
    let log_str = matches.value_of("log");
    match log_str {
        Some(l) => {
            // I know this is janky but I seriously couldn't figure out how clap want's to do this. WTF?
            let re = Regex::new(r"^(info|warn|debug|error)$").unwrap();
            let info = Regex::new(r"^info$").unwrap();
            let debug = Regex::new(r"^debug$").unwrap();
            let error = Regex::new(r"^error$").unwrap();
            if re.is_match(l) {
                if info.is_match(l)  { log_level = LevelFilter::Info; }
                if debug.is_match(l) { log_level = LevelFilter::Debug; }
                if error.is_match(l) { log_level = LevelFilter::Error; }
            }
        }
        None => { assert!(false, "If you're seeing this, parsing the log level -l argument with clap failed!"); }
    }

    CombinedLogger::init(
        vec![
            TermLogger::new(log_level, Config::default(), TerminalMode::Mixed, ColorChoice::Auto),
        ]
    ).unwrap();


    //
    // Init iio-sensor-proxy dbus connection
    //
    let sensor_service   = "net.hadess.SensorProxy";
    let sensor_path      = "/net/hadess/SensorProxy";
    
    let system_conn = Connection::system().await?;
    
    let sensor_proxy = SensorProxyProxy::new(&system_conn).await?;
    sensor_proxy.claim_light().await?;
    
    // Subscribe to PropertyChanged signal
    let props = zbus::fdo::PropertiesProxy::builder(&system_conn)
        .destination(sensor_service)?
        .path(sensor_path)?
        .build()
        .await?;

    let mut props_changed = props.receive_properties_changed().await?;

    //
    // Init Plasma desktop brightness control dbus connection
    //
    let session_conn = Connection::session().await?;
    let brightess_control_proxy = BrightnessControlProxy::new(&session_conn).await?;

    let brightness_steps = brightess_control_proxy.brightness_steps().await? as f64;
    let brightness_max = brightess_control_proxy.brightness_max().await? as f64;
    let brightness_step_size = brightness_max / brightness_steps;

    let mut max_lux = 0_usize;
    let mut min_lux = 1000_usize;
    let mut last_change_value = 0_usize;

    
    // TODO use this to adjust user selected brightness skew
    // let mut brightness_changed = brightess_control_proxy.receive_brightness_changed().await?;

    //
    // Suspend/resume events
    //
    let suspend_session_proxy = SuspendSessionProxy::new(&session_conn).await?;
    let mut resuming_signal = suspend_session_proxy.receive_resuming_from_suspend().await?;
    let mut suspending_signal = suspend_session_proxy.receive_about_to_suspend().await?;


    // TODO refactor all of this so brightness is handled in one place. Or don't, who cares? None of this needed to be async either and is probably just wasting ram.
    // This is just to set the brightness on program startup
    async fn set_brightness<'a>(sensor_proxy: &SensorProxyProxy<'_>, brightess_control_proxy: &BrightnessControlProxy<'_>, skew: f64) -> Result<(), > {
        
        let brightness_steps = brightess_control_proxy.brightness_steps().await? as f64;
        let brightness_max = brightess_control_proxy.brightness_max().await? as f64;
        let brightness_step_size = brightness_max / brightness_steps;


        let lux_value = sensor_proxy.light_level().await? as usize;
        let proposed_light_step = get_light_step(lux_value as f64, brightness_steps);
        
        // Calculate target brightness level
        let setting = (proposed_light_step * brightness_step_size * skew) as i32;
        let sensor_unit = sensor_proxy.light_level_unit().await?;
        assert!(sensor_unit=="lux");

        info!("Adjusting initial brightness setting to step {}/{}, level: {}.", proposed_light_step, brightness_steps, setting);
        brightess_control_proxy.set_brightness_silent(setting).await?;

        Ok(())
    }
    set_brightness(&sensor_proxy, &brightess_control_proxy, skew).await?;

    //
    // Handle events
    //
    let handle_light_sensor_change = async {
            while let Some(signal) = props_changed.next().await {
                let args = signal.args()?;

                for (name, _value) in args.changed_properties().iter() {
                    if name == &"LightLevel" {

                        let lux_value = sensor_proxy.light_level().await? as usize;
                        max_lux = cmp::max(lux_value, max_lux);
                        min_lux = cmp::min(lux_value, min_lux);

                        let mut should_change = false;
                        if (cmp::max(last_change_value, lux_value) - cmp::min(last_change_value, lux_value)) > sensitivity {
                            last_change_value = lux_value;
                            should_change = true;
                        }

                        // Change threshold met, changing screen brightness.
                        if should_change {

                            let current_brightness = brightess_control_proxy.brightness().await? as f64;
                            let brightness_percentage = (current_brightness / brightness_max) * 100.0;
                            let proposed_light_step = get_light_step(lux_value as f64, brightness_steps);
                            
                            // Calculate target brightness level
                            let setting = (proposed_light_step * brightness_step_size * skew) as i32;
                            let sensor_unit = sensor_proxy.light_level_unit().await?;
                            assert!(sensor_unit=="lux");

                            info!("LightLevel: {} {}, min: {}, max: {}, proposed brightness step {:2.0}/{}. Brightness: value {}, {:3.2} %, max {}. Setting to {}", 
                                lux_value, 
                                sensor_unit, 
                                min_lux,
                                max_lux,
                                proposed_light_step,
                                brightness_steps,
                                current_brightness,
                                brightness_percentage,
                                brightness_max,
                                setting
                                );

                            brightess_control_proxy.set_brightness_silent(setting).await?;
                        }
                    }
                }
            }

            Ok::<(), zbus::Error>(())
        };

    let handle_suspending_event = async {
            while let Some(_) = suspending_signal.next().await {
                sensor_proxy.release_light().await?;
                debug!("Suspending...");
            }
            Ok::<(), zbus::Error>(())
        };

    let handle_resume_event = async {
            while let Some(_) = resuming_signal.next().await {
                sensor_proxy.claim_light().await?;
                debug!("Resuming from suspend...");
            }
            Ok::<(), zbus::Error>(())
        };

    //TODO use this to adjust user selected brightness skew
    // let handle_brightness_change = async {
    //         while let Some(_) = brightness_changed.next().await {
    //             debug!("Brightness changed...");
    //         }
    //         Ok::<(), zbus::Error>(())
    //     };

    futures_util::try_join!(
        handle_light_sensor_change,
        handle_suspending_event,
        handle_resume_event,
        // handle_brightness_change,
    )?;

   Ok(())
}

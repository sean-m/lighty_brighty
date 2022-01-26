/* 
    lighty_brighty
    All the trappings of a modern laptop OS are here in Gnome and Plasma, why
    is automatic screen brightness adjustment only found in Gnome? Well DBus
    exists and those Gnome folks really love it. Ambient light sensor data that
    gnome-shell uses for automatic screen brightness is exposed by the wonderful
    iio-sensor-proxy service.

    Some simple code and Plasma's dbus inteface for screen brightness make it
    possible to have a screen brightness daemon that works without root privileges
    and only adjusts brightness when needed. This is not that service, this is
    a tribute.
*/

use std::cmp;

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
    // iio-sensor-proxy dbus connection
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
    // Plasma desktop brightness control dbus connection
    //
    let session_conn = Connection::session().await?;
    let brightess_control_proxy = BrightnessControlProxy::new(&session_conn).await?;

    let brightness_steps = brightess_control_proxy.brightness_steps().await? as f64;
    let brightness_max = brightess_control_proxy.brightness_max().await? as f64;
    let brightness_step_size = brightness_max / brightness_steps;

    let mut max_lux = 0 as usize;
    let mut min_lux = 1000 as usize;
    let mut last_change_value = 0 as usize;

    let sensitivity = 10.0 as usize; // Threshold that needs to be crossed in lux values before adjusting screen brightness. Hopefully prevents herky jerky changes. TODO: make this an argument
    let skew = 0.8 as f64; // Multiplied to the brightness setting before telling Plasma to change. 80% seems right. TODO: make this an argument

    let mut brightness_changed = brightess_control_proxy.receive_brightness_changed().await?;

    //
    // Suspend/resume events
    //
    let suspend_session_proxy = SuspendSessionProxy::new(&session_conn).await?;
    let mut resuming_signal = suspend_session_proxy.receive_resuming_from_suspend().await?;
    

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
                            

                            let setting = (proposed_light_step * brightness_step_size * skew) as i32;

                            println!("LightLevel : {} {}, max: {}, min: {}, proposed step: {}.\tBrightness value: {},  {} %, steps: {}, max: {}.\t Setting to: {}", 
                                lux_value, 
                                sensor_proxy.light_level_unit().await?, 
                                max_lux,
                                min_lux,
                                proposed_light_step,
                                current_brightness,
                                brightness_percentage,
                                brightness_steps,
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



    let handle_resume_event = async {
            while let Some(_) = resuming_signal.next().await {
                println!("Resuming from suspend...");
            }
            Ok::<(), zbus::Error>(())
        };

    let handle_brightness_change = async {
            while let Some(_) = brightness_changed.next().await {
                println!("Brightness changed...");
            }
            Ok::<(), zbus::Error>(())
        };

    futures_util::try_join!(
        handle_light_sensor_change,
        handle_resume_event,
        handle_brightness_change,
    )?;

   Ok(())
}

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

use crate::kde_brightness_control_trait::BrightnessControlProxy;
use crate::sensor_proxy_trait::SensorProxyProxy;
use zbus::{Connection, Result};
use futures_util::stream::StreamExt;


mod sensor_proxy_trait;
mod kde_brightness_control_trait;

fn get_light_step(lux_value : f64, step_count : f64) -> f64 {
    let lux = lux_value;
    let log_step = lux.log10() / 5.0;
    println!("log_step {}", log_step);
    (step_count * log_step).max(1.0)
}

// Although we use `async-std` here, you can use any async runtime of choice.
#[async_std::main]
async fn main() -> Result<()> {

    // iio-sensor-proxy dbus connection
    let _sensor_service   = "net.hadess.SensorProxy";
    let _sensor_path      = "/net/hadess/SensorProxy";

    let system_conn = Connection::system().await?;
    let session_conn = Connection::session().await?;

    let sensor_proxy = SensorProxyProxy::new(&system_conn).await?;
    sensor_proxy.claim_light().await?;
    

    // Plasma desktop brightness control dbus connection
    let brightess_control_service = "org.kde.Solid.PowerManagement.Actions.BrightnessControl";
    let brightess_control_path = "/org/kde/Solid/PowerManagement/Actions/BrightnessControl";

    let brightess_control_proxy = BrightnessControlProxy::new(&session_conn).await?;
    let _brightness_steps = brightess_control_proxy.brightness_steps().await? as f64;
        
    let brightness_props = zbus::fdo::PropertiesProxy::builder(&session_conn)
        .destination(brightess_control_service)?
        .path(brightess_control_path)?
        .build()
        .await?;
    

    let mut brightness_changed = brightness_props.receive_properties_changed().await?;

    futures_util::try_join!(
        async {
            while let Some(signal) = brightness_changed.next().await {
                let args = signal.args()?;

                for (name, value) in args.changed_properties().iter() {
                    println!("Property : {}, {:?}", name, value);
                }
            }

            Ok::<(), zbus::Error>(())
        }
    )?;

   Ok(())
}

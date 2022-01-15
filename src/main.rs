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


use crate::sensor_proxy_trait::SensorProxyProxy;
use zbus::{Connection, Result};
use futures_util::stream::StreamExt;


mod sensor_proxy_trait;
mod kde_brightness_control_trait;


// Although we use `async-std` here, you can use any async runtime of choice.
#[async_std::main]
async fn main() -> Result<()> {

    let sensor_service   = "net.hadess.SensorProxy";
    let sensor_path      = "/net/hadess/SensorProxy";

    let conn = Connection::system().await?;
    let proxy = SensorProxyProxy::new(&conn).await?;
    proxy.claim_light().await?;
    
    let props = zbus::fdo::PropertiesProxy::builder(&conn)
        .destination(sensor_service)?
        .path(sensor_path)?
        .build()
        .await?;

    let mut props_changed = props.receive_properties_changed().await?;


    futures_util::try_join!(
        async {
            while let Some(signal) = props_changed.next().await {
                let args = signal.args()?;

                for (name, _value) in args.changed_properties().iter() {
                    if name == &"LightLevel" {

                        println!("LightLevel : {} {}", proxy.light_level().await?, proxy.light_level_unit().await?);    
                    }
                }
            }

            Ok::<(), zbus::Error>(())
        }
    )?;

   Ok(())
}

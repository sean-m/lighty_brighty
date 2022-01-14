use zbus::{Connection, dbus_proxy, Result};
use zbus::zvariant::ObjectPath;
use futures_util::stream::StreamExt;

#[dbus_proxy(
    default_service = "org.freedesktop.GeoClue2",
    interface = "org.freedesktop.GeoClue2.Manager",
    default_path = "/org/freedesktop/GeoClue2/Manager"
)]
trait Manager {
    #[dbus_proxy(object = "Client")]
    fn get_client(&self);
}

#[dbus_proxy(
    default_service = "org.freedesktop.GeoClue2",
    interface = "org.freedesktop.GeoClue2.Client"
)]
trait Client {
    fn start(&self) -> Result<()>;
    fn stop(&self) -> Result<()>;

    #[dbus_proxy(property)]
    fn set_desktop_id(&mut self, id: &str) -> Result<()>;

    #[dbus_proxy(signal)]
    fn location_updated(&self, old: ObjectPath<'_>, new: ObjectPath<'_>) -> Result<()>;
}

#[dbus_proxy(
    default_service = "org.freedesktop.GeoClue2",
    interface = "org.freedesktop.GeoClue2.Location"
)]
trait Location {
    #[dbus_proxy(property)]
    fn latitude(&self) -> Result<f64>;
    #[dbus_proxy(property)]
    fn longitude(&self) -> Result<f64>;
}

// Although we use `async-std` here, you can use any async runtime of choice.
#[async_std::main]
async fn main() -> Result<()> {
    let conn = Connection::system().await?;
    let manager = ManagerProxy::new(&conn).await?;
    let mut client = manager.get_client().await?;
    // Gotta do this, sorry!
    client.set_desktop_id("org.freedesktop.zbus").await?;

    let props = zbus::fdo::PropertiesProxy::builder(&conn)
        .destination("org.freedesktop.GeoClue2")?
        .path(client.path())?
        .build()
        .await?;
    let mut props_changed = props.receive_properties_changed().await?;
    let mut location_updated = client.receive_location_updated().await?;

    client.start().await?;

    futures_util::try_join!(
        async {
            while let Some(signal) = props_changed.next().await {
                let args = signal.args()?;

                for (name, value) in args.changed_properties().iter() {
                    println!("{}.{} changed to `{:?}`", args.interface_name(), name, value);
                }
            }

            Ok::<(), zbus::Error>(())
        },
        async {
            while let Some(signal) = location_updated.next().await {
                let args = signal.args()?;

                let location = LocationProxy::builder(&conn)
                    .path(args.new())?
                    .build()
                    .await?;
                println!(
                    "Latitude: {}\nLongitude: {}",
                    location.latitude().await?,
                    location.longitude().await?,
                );
            }

            // No need to specify type of Result each time
            Ok(())
        }
    )?;

   Ok(())
}

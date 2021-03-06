//! # DBus interface proxy for: `net.hadess.SensorProxy`
//!
//! This code was generated by `zbus-xmlgen` `2.0.1` from DBus introspection data.
//! Source: `Interface '/net/hadess/SensorProxy' from service 'net.hadess.SensorProxy' on system bus`.
//!
//! You may prefer to adapt it, instead of using it verbatim.
//!
//! More information can be found in the
//! [Writing a client proxy](https://dbus.pages.freedesktop.org/zbus/client.html)
//! section of the zbus documentation.
//!
//! This DBus object implements
//! [standard DBus interfaces](https://dbus.freedesktop.org/doc/dbus-specification.html),
//! (`org.freedesktop.DBus.*`) for which the following zbus proxies can be used:
//!
//! * [`zbus::fdo::PropertiesProxy`]
//! * [`zbus::fdo::IntrospectableProxy`]
//! * [`zbus::fdo::PeerProxy`]
//!
//! …consequently `zbus-xmlgen` did not generate code for the above interfaces.

use zbus::dbus_proxy;

#[dbus_proxy(
    default_service = "net.hadess.SensorProxy",
    interface = "net.hadess.SensorProxy",
    default_path = "/net/hadess/SensorProxy",
)]
trait SensorProxy {
    /// ClaimAccelerometer method
    fn claim_accelerometer(&self) -> zbus::Result<()>;

    /// ClaimLight method
    fn claim_light(&self) -> zbus::Result<()>;

    /// ClaimProximity method
    fn claim_proximity(&self) -> zbus::Result<()>;

    /// ReleaseAccelerometer method
    fn release_accelerometer(&self) -> zbus::Result<()>;

    /// ReleaseLight method
    fn release_light(&self) -> zbus::Result<()>;

    /// ReleaseProximity method
    fn release_proximity(&self) -> zbus::Result<()>;

    /// AccelerometerOrientation property
    #[dbus_proxy(property)]
    fn accelerometer_orientation(&self) -> zbus::Result<String>;

    /// HasAccelerometer property
    #[dbus_proxy(property)]
    fn has_accelerometer(&self) -> zbus::Result<bool>;

    /// HasAmbientLight property
    #[dbus_proxy(property)]
    fn has_ambient_light(&self) -> zbus::Result<bool>;

    /// HasProximity property
    #[dbus_proxy(property)]
    fn has_proximity(&self) -> zbus::Result<bool>;

    /// LightLevel property
    #[dbus_proxy(property)]
    fn light_level(&self) -> zbus::Result<f64>;

    /// LightLevelUnit property
    #[dbus_proxy(property)]
    fn light_level_unit(&self) -> zbus::Result<String>;

    /// ProximityNear property
    #[dbus_proxy(property)]
    fn proximity_near(&self) -> zbus::Result<bool>;
}

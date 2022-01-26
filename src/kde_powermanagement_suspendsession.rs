//! # DBus interface proxy for: `org.kde.Solid.PowerManagement.Actions.SuspendSession`
//!
//! This code was generated by `zbus-xmlgen` `2.0.1` from DBus introspection data.
//! Source: `Interface '/org/kde/Solid/PowerManagement/Actions/SuspendSession' from service 'org.freedesktop.PowerManagement' on session bus`.
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
    default_service = "org.kde.Solid.PowerManagement",
    default_path = "/org/kde/Solid/PowerManagement/Actions/SuspendSession",
    interface = "org.kde.Solid.PowerManagement.Actions.SuspendSession"
)]
trait SuspendSession {
    /// suspendHybrid method
    fn suspend_hybrid(&self) -> zbus::Result<()>;

    /// suspendToDisk method
    fn suspend_to_disk(&self) -> zbus::Result<()>;

    /// suspendToRam method
    fn suspend_to_ram(&self) -> zbus::Result<()>;

    /// aboutToSuspend signal
    #[dbus_proxy(signal, name="aboutToSuspend")]
    fn about_to_suspend(&self) -> zbus::Result<()>;

    /// resumingFromSuspend signal
    #[dbus_proxy(signal, name="resumingFromSuspend")]
    fn resuming_from_suspend(&self) -> zbus::Result<()>;
}
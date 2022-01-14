# What is this?
Should be a program that scales your screen brightness based on the ambient light sensor reading

As a stretch goal it should remember what you set the brightness to at a given sensor reading (if you change it manually), and set that as the anchor value for the desired brightness at that sensor reading. Brightness levels are then lerped through the anchor values. Will need to set a range within some proximity to a linreg of the sensor/manual values so it wont learn a value that's way out of whack. 

Plasma desktop should just do this.

It's gonna use DBus and the iio-sensor-proxy from the Gnome project but really be targeting use in KDE. Not sure if it'll be a Systemd service (likely since I wanna do it in Rust), or a KDE plasmid (not excited about javascript but oh well), maybe just an autostarting executable?

# Links
[net.hadess.SensorProxy from Gnome project](https://developer-old.gnome.org/iio-sensor-proxy/2.3/gdbus-net.hadess.SensorProxy.html#gdbus-method-net-hadess-SensorProxy.ClaimLight)
[KDE power management screen brightness dbus](https://userbase.kde.org/KDE_Connect/Tutorials/Useful_commands#Brightness_settings)
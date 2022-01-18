#!/usr/bin/env sh



dbus_interface="org.kde.Solid.PowerManagement"
dbus_addr_path="/org/kde/Solid/PowerManagement/Actions/BrightnessControl"
dbus_addr_pow_bright_ctrl="${dbus_interface}.Actions.BrightnessControl"

increase() {

	qdbus ${dbus_interface} ${dbus_addr_path} "${dbus_addr_pow_bright_ctrl}.setBrightness" $(expr $(qdbus ${dbus_ns} ${dbus_addr_interface} "${dbus_addr_pow_bright_ctrl}.brightness") + $1)
}

decrease() {

	qdbus ${dbus_interface} ${dbus_addr_path} "${dbus_addr_pow_bright_ctrl}.setBrightness" $(expr $(qdbus ${dbus_ns} ${dbus_addr_interface} "${dbus_addr_pow_bright_ctrl}.brightness") - $1)
}

#qdbus org.kde.Solid.PowerManagement /org/kde/Solid/PowerManagement/Actions/BrightnessControl org.kde.Solid.PowerManagement.Actions.BrightnessControl.brightness

current_brightness=`qdbus ${dbus_ns} ${dbus_addr_interface} "${dbus_addr_pow_bright_ctrl}.brightness"`

if [ $current_brightness -lt 96000 ]; then increase 5000; else echo "Nope"; fi


qdbus ${dbus_interface} ${dbus_addr_path} "${dbus_addr_pow_bright_ctrl}.brightness"


# qdbus net.hadess.SensorProxy /net/hadess/SensorProxy ClaimLight
# zbus-xmlgen --system net.hadess.SensorProxy /net/hadess/SensorProxy
# zbus-xmlgen --system org.kde.Solid.PowerManagement /org/kde/Solid/PowerManagement/Actions/BrightnessControl
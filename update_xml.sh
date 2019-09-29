#! /bin/bash

dbus-send --system \
	--dest=org.freedesktop.systemd1 \
	--type=method_call \
	--print-reply=literal \
	/org/freedesktop/systemd1 \
	org.freedesktop.DBus.Introspectable.Introspect > systemd-manager.xml

dbus-send --system \
	--dest=org.freedesktop.systemd1 \
	--type=method_call \
	--print-reply=literal \
	/org/freedesktop/systemd1/unit/dbus_2eservice \
	org.freedesktop.DBus.Introspectable.Introspect > systemd-service.xml

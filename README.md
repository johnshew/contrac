# Contrac

Contrac monitors your Internet Service Provider and generates a log of down time.

![](contrac.png?raw=true)

The tracker pings Google, Cloudflare, and Cisco to determine connectivity. The app is a single file .exe. It has embedded resources, tray notifications, and supports window minimization to the tray area.

## Installation

Copy the latest release of the exe to your computer from here

https://github.com/johnshew/contrac/releases

Since Contrac automatically writes to the log files every 5 minutes it is good to put the exe in the directory you want the logs to go.

## Notes

This was an interesting first project to learn Rust. Thanks to [Gabriel Dube](https://github.com/gabdube) for creating native-windows-gui, a nice toolkit for small native Win32 apps.

## Recommended Visual Studio Extensions

* https://marketplace.visualstudio.com/items?itemName=rust-lang.rust
* https://marketplace.visualstudio.com/items?itemName=vadimcn.vscode-lldb

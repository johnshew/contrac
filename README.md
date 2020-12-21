# Contrac

Contrac monitors your Internet Service Provider and generates a log of down time.

![](contrac.png?raw=true)

The tracker pings Cloudflare, Cisco OpenDNS, Google, and Quad9 to determine connectivity. No service is pinged more than once per second.  

The app is a single file .exe with has embedded resources, tray notifications, and supports minimization to the tray area.  It is packaged as an MSIX to enable easy installation and clean up.

## Installation

Copy the latest release of the .msix or .exe to your computer from here

https://github.com/johnshew/contrac/releases

## Notes

This was an interesting first project to learn Rust. Thanks to [Gabriel Dube](https://github.com/gabdube) for creating native-windows-gui, a nice toolkit for small native Win32 apps.

## Recommended Visual Studio Extensions

* https://marketplace.visualstudio.com/items?itemName=rust-lang.rust
* https://marketplace.visualstudio.com/items?itemName=vadimcn.vscode-lldb

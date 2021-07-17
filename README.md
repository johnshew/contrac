# Contrac

Contrac monitors your Internet Service Provider and generates a log of down time in your documents folder.

![](contrac.png?raw=true)

The tracker pings Cloudflare, Cisco OpenDNS, Google, and Quad9 to determine connectivity. No service is pinged more than once per second.  

The app is a single file .exe with embedded resources. It supports tray notifications and minimization to the tray area.  It is packaged as an MSIX to enable easy installation and clean up.  It is available in the Microsoft Store on Windows.

## Installation

The easist way to install it is from the Microsoft Store on Windows.  The url is:

https://www.microsoft.com/store/apps/9P7D2CC6Q9DH

Alternatively you can get the latest release of the .msix or .exe and run it:

https://github.com/johnshew/contrac/releases

## Notes

This was an interesting first project to learn Rust. Thanks to [Gabriel Dube](https://github.com/gabdube) for creating native-windows-gui, a nice toolkit for small native Win32 apps.

## Recommended Visual Studio Extensions for Development

* https://marketplace.visualstudio.com/items?itemName=rust-lang.rust
* https://marketplace.visualstudio.com/items?itemName=vadimcn.vscode-lldb

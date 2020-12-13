# Contrac

Contrac monitors your Internet Service Provider and generates a log of down time.

This tracker pings Google, Cloudflare, and Cisco to determine connectivity.

This windows app is a single file .exe, with embedded resources, tray notification, and window minimization to the tray.

## Installation

Copy the latest release of the exe to your computer from here

https://github.com/johnshew/contrac/releases

Since Contrac automatically writes to the log files every 5 minutes it is good to put the exe in the directory you want the logs to go.

## Notes

This was an interesting first project to learn Rust. Thanks to [Gabriel Dube](https://github.com/gabdube) for creating native-windows-gui, a nice toolkit for small native Win32 apps.

The graph rendering in Contrac is a hack.  Since there are currently no easy-to-use drawing capabilties with native-windows-gui, Contrac uses a collection of small image controls to represent the bars of the graph.

On Windows, by default, Rust starts a console.  If you want a Windows app put the following at the top of main.rs.

``` #![windows_subsystem = "windows"] ```

Here a few things that I thought about PRing into native-windows-gui:
* Minimize and restore windows
* Maybe enable WS_CLIPSIBLINGS in flags for controls and enable z-ordering
* Provide more access to the nwg::win32 helper functions

## Recommended Visual Studio Extensions

* https://marketplace.visualstudio.com/items?itemName=rust-lang.rust
* https://marketplace.visualstudio.com/items?itemName=vadimcn.vscode-lldb

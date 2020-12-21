# Contrac Developer Notes


* The graph rendering in Contrac is a hack.  Since there are currently no easy-to-use drawing capabilties with native-windows-gui, Contrac uses a collection of small image controls to represent the bars of the graph.

* On Windows, by default, Rust starts a console.  If you want a Windows app put the following at the top of main.rs.
``` #![windows_subsystem = "windows"] ```

Here a few things that I thought about PRing into native-windows-gui:
* Minimize and restore windows
* Maybe enable WS_CLIPSIBLINGS in flags for controls and enable z-ordering
* Provide more access to the nwg::win32 helper functions
* Add utility functions to edit box for scrolling

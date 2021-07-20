# Contrac Developer Notes

# Building and deploying 

## MSIX and getting it to the Store

* Need to create the MSIX first
  * Update the version number in corgo.toml
  * Update the version number in AAI
  * x64
    * cargo build --release 
    * set x64 in AAI and build MSIX
  * x86
    * rustup run stable-i686-pc-windows-msvc cargo build --release
    * set x86 in AAI and build MSIX
  * To run locally you might need to create a new cert and set it.
  * Remember that the packager takes the release not debug target
  * Remember to build both x86 and x64 versions

* Bundle up the files to github and label the release
  * Drag the two MSIX and maybe two exes to the release
  * Add a tag of the form v0.n.0

* Submit to store
  * https://partner.microsoft.com/en-us/dashboard/windows/overview
  * Remember to update the release notes

## To build x86 32-bit app
```
rustup run stable-i686-pc-windows-msvc cargo build --release
```
In Advanced Application Install go to Package Definitions / Builds and set to x86

# Overall Approach and Learnings
On Windows, by default, Rust starts a console.  If you want a Windows app put the following at the top of main.rs.
```
#![windows_subsystem = "windows"] 
```

The graph rendering in Contrac is a hack.  Since there are currently no easy-to-use drawing capabilties with native-windows-gui, Contrac uses a collection of small image controls to represent the bars of the graph.  That said, there maybe now a graphing library.  That has the potential to remove a lot of wonky code.


## Things that might be useful to PR into native-windows-gui:
  * Minimize and restore windows functionality that is working
  * Maybe enable WS_CLIPSIBLINGS in flags for controls and enable z-ordering
  * Provide more access to the nwg::win32 helper functions
  * Add utility functions to edit box for scrolling


## Recommended Visual Studio Extensions for Development

* https://marketplace.visualstudio.com/items?itemName=rust-lang.rust
* https://marketplace.visualstudio.com/items?itemName=vadimcn.vscode-lldb

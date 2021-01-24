# compressor-plugin (Experimental/WIP)
A compressor vst plugin in rust, using imgui. 

The plugin logs events to `~/tmp/IMGUIBaseviewCompressor.log`.

Parameters for RMS & pre-smoothing have not been added to the GUI yet. Use your DAW's GUI-less mode to access these parameters.

This plugin is in very early stages of development. Until version 1.0, parameters will change and compatibility will not be kept between updates. 

![Demo](demo.png)

## Usage: macOS (Untested)

- Run `scripts/macos-build-and-install.sh`
- Start your DAW, test the plugin

## Usage: Windows

- Run `cargo build`
- Copy `target/debug/imgui_baseview_compressor.dll` to your VST plugin folder
- Start your DAW, test the plugin

## Usage: Linux (Untested)

- Run `cargo build`
- Copy `target/debug/imgui_baseview_compressor.so` to your VST plugin folder
- Start your DAW, test the plugin


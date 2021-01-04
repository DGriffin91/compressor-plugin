# rust-imgui-compressor-plugin (Experimental/WIP)
A compressor vst plugin in rust, using imgui. 

The plugin logs events to `~/tmp/IMGUIBaseviewCompressor.log`.

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


name := "papery"
appid := "dev.papery.CosmicApplet"
plugin_id := "dev.papery.wallpaper"
prefix := "/usr/local"
bindir := prefix / "bin"
sharedir := prefix / "share"
iconsdir := sharedir / "icons/hicolor"
plasma_plugin_dir := env_var('HOME') / ".local/share/plasma/wallpapers" / plugin_id

build:
    cargo build --release

run:
    cargo run

install: build
    install -Dm0755 target/release/{{name}} {{bindir}}/{{name}}
    install -Dm0644 data/{{appid}}.desktop {{sharedir}}/applications/{{appid}}.desktop
    install -Dm0644 data/{{appid}}-autostart.desktop ~/.config/autostart/{{appid}}.desktop
    install -Dm0644 data/icons/scalable/apps/{{appid}}.svg {{iconsdir}}/scalable/apps/{{appid}}.svg

# Install the KDE Plasma wallpaper plugin so Papery shows up in the
# "Wallpaper type" dropdown of System Settings > Wallpaper.
install-kde-plugin:
    mkdir -p "{{plasma_plugin_dir}}/contents/ui"
    install -Dm0644 data/kde-plugin/metadata.json "{{plasma_plugin_dir}}/metadata.json"
    install -Dm0644 data/kde-plugin/contents/ui/main.qml "{{plasma_plugin_dir}}/contents/ui/main.qml"
    install -Dm0644 data/kde-plugin/contents/ui/config.qml "{{plasma_plugin_dir}}/contents/ui/config.qml"
    @echo "Installed Papery wallpaper plugin to {{plasma_plugin_dir}}"
    @echo "Re-open System Settings > Wallpaper and pick 'Papery' from the dropdown."

uninstall:
    rm -f {{bindir}}/{{name}}
    rm -f {{sharedir}}/applications/{{appid}}.desktop
    rm -f ~/.config/autostart/{{appid}}.desktop
    rm -f {{iconsdir}}/scalable/apps/{{appid}}.svg

uninstall-kde-plugin:
    rm -rf "{{plasma_plugin_dir}}"

clean:
    cargo clean

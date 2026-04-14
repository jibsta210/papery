name := "papery"
appid := "dev.papery.CosmicApplet"
prefix := "/usr/local"
bindir := prefix / "bin"
sharedir := prefix / "share"
iconsdir := sharedir / "icons/hicolor"

build:
    cargo build --release

run:
    cargo run

install: build
    install -Dm0755 target/release/{{name}} {{bindir}}/{{name}}
    install -Dm0644 data/{{appid}}.desktop {{sharedir}}/applications/{{appid}}.desktop
    install -Dm0644 data/{{appid}}-autostart.desktop ~/.config/autostart/{{appid}}.desktop
    install -Dm0644 data/icons/scalable/apps/{{appid}}.svg {{iconsdir}}/scalable/apps/{{appid}}.svg

uninstall:
    rm -f {{bindir}}/{{name}}
    rm -f {{sharedir}}/applications/{{appid}}.desktop
    rm -f ~/.config/autostart/{{appid}}.desktop
    rm -f {{iconsdir}}/scalable/apps/{{appid}}.svg

clean:
    cargo clean

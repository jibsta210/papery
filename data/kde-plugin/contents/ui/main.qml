/*
 * Papery wallpaper plugin for KDE Plasma 6
 * SPDX-License-Identifier: GPL-3.0-only
 *
 * Renders ~/.cache/papery/current.jpg — a symlink that Papery's daemon
 * keeps pointed at the active wallpaper. Every 3 seconds we ask Plasma's
 * shell engine to stat() the symlink and tell us its mtime. When the mtime
 * changes we bump a revision counter that forces the Image element (with
 * cache:false) to reload from disk.
 *
 * Why stat over reading file content: a `stat` is a single syscall vs.
 * decoding a multi-megabyte file every poll, and unlike QML XMLHttpRequest
 * for file:// URLs it actually reports changes reliably.
 */
import QtCore
import QtQuick
import org.kde.plasma.core as PlasmaCore
import org.kde.plasma.plasma5support as P5Support
import org.kde.kirigami as Kirigami
import org.kde.plasma.plasmoid

WallpaperItem {
    id: root

    readonly property string cacheDir: StandardPaths.writableLocation(StandardPaths.GenericCacheLocation).toString().replace("file://", "")
    readonly property string wallpaperFile: cacheDir + "/papery/current.jpg"
    readonly property string triggerFile: cacheDir + "/papery/trigger_next"
    readonly property string statCmd: "stat -L -c %Y%n%s " + wallpaperFile + " 2>/dev/null"

    // Bumped whenever the symlinked file's mtime changes — forces Image reload.
    property int wallpaperRevision: 0
    property string lastFingerprint: ""

    contextualActions: [
        PlasmaCore.Action {
            text: i18n("Next Wallpaper")
            icon.name: "media-skip-forward-symbolic"
            onTriggered: Qt.callLater(triggerNext)
        },
        PlasmaCore.Action {
            text: i18n("Open Papery")
            icon.name: "preferences-desktop-wallpaper"
            onTriggered: Qt.callLater(openPapery)
        }
    ]

    Component.onCompleted: {
        console.log("[Papery] plugin loaded; wallpaperFile =", wallpaperFile)
    }

    Rectangle {
        anchors.fill: parent
        color: "#1d1d1d"
    }

    Image {
        id: wallpaperImage
        anchors.fill: parent
        asynchronous: true
        cache: false
        smooth: true
        mipmap: true
        fillMode: switch (configuration.FillMode) {
            case 0: return Image.Stretch
            case 1: return Image.PreserveAspectFit
            case 3: return Image.Tile
            default: return Image.PreserveAspectCrop
        }
        source: "file://" + wallpaperFile + "?v=" + wallpaperRevision
        onStatusChanged: {
            if (status === Image.Error) {
                console.log("[Papery] Image error:", source)
            } else if (status === Image.Ready) {
                console.log("[Papery] rendered (rev", wallpaperRevision, ")")
            }
        }
    }

    // ---- File watcher via stat -------------------------------------------

    P5Support.DataSource {
        id: stat
        engine: "executable"
        connectedSources: [statCmd]
        interval: 3000

        onNewData: (sourceName, data) => {
            const stdout = (data["stdout"] || "").trim()
            if (stdout.length === 0) {
                console.log("[Papery] stat empty (symlink missing?)")
                return
            }
            if (stdout !== lastFingerprint) {
                console.log("[Papery] file changed:", stdout)
                lastFingerprint = stdout
                wallpaperRevision++
            }
        }
    }

    // Belt-and-braces: even if the stat datasource breaks silently, force a
    // reload every 60s so the wallpaper still picks up changes (worst case
    // a 1-minute lag instead of being stuck forever).
    Timer {
        interval: 60000
        running: true
        repeat: true
        onTriggered: wallpaperRevision++
    }

    // ---- Action handlers --------------------------------------------------

    function triggerNext() {
        const xhr = new XMLHttpRequest()
        xhr.open("PUT", "file://" + triggerFile)
        try { xhr.send("1") } catch (e) { console.log("[Papery] trig err:", e) }
    }

    function openPapery() {
        Qt.openUrlExternally("file:///usr/local/bin/papery")
    }
}

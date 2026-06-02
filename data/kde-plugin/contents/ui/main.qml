/*
 * Papery wallpaper plugin for KDE Plasma 6
 * SPDX-License-Identifier: GPL-3.0-only
 *
 * Renders ~/.cache/papery/current.jpg — a symlink Papery's daemon keeps
 * pointed at the active wallpaper. Polls every 2s and bumps a cache-busting
 * revision counter when the file size changes so the Image element reloads.
 */
import QtCore
import QtQuick
import org.kde.plasma.core as PlasmaCore
import org.kde.kirigami as Kirigami
import org.kde.plasma.plasmoid

WallpaperItem {
    id: root

    readonly property string cacheDir: StandardPaths.writableLocation(StandardPaths.GenericCacheLocation).toString().replace("file://", "")
    readonly property string wallpaperFile: cacheDir + "/papery/current.jpg"
    readonly property string triggerFile: cacheDir + "/papery/trigger_next"

    property int wallpaperRevision: 0
    property real lastSize: -1

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
        checkForChange()
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

    Timer {
        interval: 2000
        running: true
        repeat: true
        onTriggered: checkForChange()
    }

    // Polls the file size via a HEAD-style XHR. file:// gives size in the
    // Content-Length header on Qt's implementation.
    function checkForChange() {
        const xhr = new XMLHttpRequest()
        xhr.open("GET", "file://" + wallpaperFile)
        xhr.responseType = "arraybuffer"
        xhr.onreadystatechange = function() {
            if (xhr.readyState !== XMLHttpRequest.DONE) return
            const size = xhr.response ? xhr.response.byteLength : -1
            if (size > 0 && size !== lastSize) {
                lastSize = size
                wallpaperRevision++
            }
        }
        try { xhr.send() } catch (e) { console.log("[Papery] poll err:", e) }
    }

    function triggerNext() {
        const xhr = new XMLHttpRequest()
        xhr.open("PUT", "file://" + triggerFile)
        try { xhr.send("1") } catch (e) { console.log("[Papery] trig err:", e) }
    }

    function openPapery() {
        Qt.openUrlExternally("file:///usr/local/bin/papery")
    }
}

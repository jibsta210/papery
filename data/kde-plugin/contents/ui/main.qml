/*
 * Papery wallpaper plugin for KDE Plasma 6
 * SPDX-License-Identifier: GPL-3.0-only
 *
 * Reads the path of the currently-active Papery wallpaper from
 * ~/.cache/papery/current_path (a one-line text file written by the daemon)
 * and renders it. Polls every 2s for changes so rotation is picked up live.
 */
import QtQuick
import QtQuick.Layouts
import org.kde.plasma.core as PlasmaCore
import org.kde.kirigami as Kirigami
import org.kde.plasma.plasmoid

WallpaperItem {
    id: root

    // Where Papery writes the active wallpaper path
    readonly property string statePath: Qt.resolvedUrl(
        "file://" + StandardPaths.writableLocation(StandardPaths.GenericCacheLocation)
                  + "/papery/current_path"
    )

    // Cache-busting counter so the Image element re-fetches when the file changes
    property int wallpaperRevision: 0
    property string currentPath: ""

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

    // ---- Rendering --------------------------------------------------------

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
            case 4: return Image.PreserveAspectFit  // centered (handled below)
            default: return Image.PreserveAspectCrop
        }
        source: currentPath.length > 0
                ? ("file://" + currentPath + "?v=" + wallpaperRevision)
                : ""
    }

    // Light fade-in when the source changes
    Behavior on opacity { NumberAnimation { duration: 400 } }

    // ---- File watcher (polling) ------------------------------------------

    Timer {
        interval: 2000
        running: true
        repeat: true
        triggeredOnStart: true
        onTriggered: refreshPath()
    }

    function refreshPath() {
        const xhr = new XMLHttpRequest()
        xhr.open("GET", statePath)
        xhr.onreadystatechange = function() {
            if (xhr.readyState !== XMLHttpRequest.DONE) return
            if (xhr.status !== 200 && xhr.status !== 0) return
            const newPath = xhr.responseText.trim()
            if (newPath.length > 0 && newPath !== currentPath) {
                currentPath = newPath
                wallpaperRevision++
            }
        }
        xhr.send()
    }

    // ---- Action handlers --------------------------------------------------

    function triggerNext() {
        // Touch a trigger file the Papery daemon watches.
        const triggerPath = StandardPaths.writableLocation(StandardPaths.GenericCacheLocation)
                          + "/papery/trigger_next"
        const xhr = new XMLHttpRequest()
        xhr.open("PUT", "file://" + triggerPath)
        xhr.send("1")
    }

    function openPapery() {
        Qt.openUrlExternally("file:///usr/local/bin/papery")
    }
}

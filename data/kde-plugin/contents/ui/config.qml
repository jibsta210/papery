/*
 * Papery plugin config UI
 * Lets the user pick how the image fills the screen.
 */
import QtQuick
import QtQuick.Layouts
import QtQuick.Controls as QQC2
import org.kde.kirigami as Kirigami

Kirigami.FormLayout {
    id: root

    property int cfg_FillMode: 2 // PreserveAspectCrop (zoom) by default

    QQC2.ComboBox {
        Kirigami.FormData.label: i18n("Positioning:")
        currentIndex: indexFromValue(cfg_FillMode)
        textRole: "text"
        valueRole: "value"
        model: [
            { value: 0, text: i18n("Stretched") },
            { value: 1, text: i18n("Scaled, keep proportions") },
            { value: 2, text: i18n("Scaled and cropped") },
            { value: 3, text: i18n("Tiled") }
        ]
        onActivated: cfg_FillMode = currentValue
        function indexFromValue(v) {
            for (let i = 0; i < model.length; i++)
                if (model[i].value === v) return i
            return 2
        }
    }

    Kirigami.Heading {
        level: 4
        text: i18n("Papery")
    }

    QQC2.Label {
        Layout.fillWidth: true
        wrapMode: Text.WordWrap
        text: i18n("Papery rotates wallpapers from online sources like Bing, NASA APOD, Wallhaven, Pexels, and more.\n\nOpen the Papery app from your application launcher to configure sources, rotation interval, and theme filtering.")
    }
}

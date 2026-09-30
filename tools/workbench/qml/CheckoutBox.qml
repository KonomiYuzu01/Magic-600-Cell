import QtQuick
import QtQuick.Controls

// A checkout chooser that keeps the owner's choice by path when the checkout list changes. If the chosen
// checkout disappears, it says so and `present` turns false; it never falls back to another checkout.
ComboBox {
    id: box
    property string path: ""
    readonly property bool present: path !== "" && currentIndex >= 0 && currentValue === path
    function sync() {
        if (path === "" && count > 0)
            path = valueAt(0)  // the main checkout, listed first
        currentIndex = indexOfValue(path)
    }
    model: wb.checkouts
    textRole: "name"
    valueRole: "path"
    displayText: present ? currentText : "The chosen checkout is gone; choose another"
    onActivated: (index) => { path = valueAt(index) }
    onModelChanged: Qt.callLater(box.sync)
    Component.onCompleted: sync()
}

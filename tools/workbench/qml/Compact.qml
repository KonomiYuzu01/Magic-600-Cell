import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

// A top-level window, so it stays visible when the main window is minimized.
Window {
    id: compact
    property bool standalone: false
    transientParent: null
    width: 420
    height: body.implicitHeight + 20
    x: Screen.desktopAvailableWidth - width - 24
    y: 24
    flags: (standalone ? Qt.Window : Qt.Tool) | Qt.WindowStaysOnTopHint | Qt.FramelessWindowHint
    title: "Workbench progress"
    color: "#202124"

    DragHandler { onActiveChanged: if (active) compact.startSystemMove() }

    ColumnLayout {
        id: body
        anchors.fill: parent
        anchors.margins: 10
        spacing: 3
        RowLayout {
            Layout.fillWidth: true
            Label { objectName: "compactStep"; text: wb.compact.stepText; color: "white"; font.bold: true; elide: Text.ElideRight; Layout.fillWidth: true }
            ToolButton { text: "×"; palette.buttonText: "white"; onClicked: compact.standalone ? Qt.quit() : compact.close() }
        }
        Label {
            objectName: "compactCounts"
            text: "For you " + wb.compact.forYou + " · Gallery +" + wb.compact.galleryNew
            color: "#e0e0e0"
        }
        Label {
            objectName: "compactApi"
            visible: wb.compact.apiVisible
            text: "Paid API " + wb.compact.apiText
            color: "#e0e0e0"; elide: Text.ElideRight; Layout.fillWidth: true
            ToolTip.visible: hover.hovered; ToolTip.text: wb.compact.apiDetail || ""
            HoverHandler { id: hover }
        }
    }
}

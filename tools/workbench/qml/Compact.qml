import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

// Always-on-top progress view. Loaded on its own with --compact (standalone: closing it quits the
// app), or opened from the main window. It is always a top-level window: a transient child of a
// hidden or minimized main window would never be shown.
Window {
    id: compact
    property bool standalone: false
    transientParent: null
    width: 420
    height: 118
    x: Screen.desktopAvailableWidth - width - 24
    y: 24
    flags: (standalone ? Qt.Window : Qt.Tool) | Qt.WindowStaysOnTopHint | Qt.FramelessWindowHint
    title: "Workbench progress"
    color: "#202124"

    DragHandler { onActiveChanged: if (active) compact.startSystemMove() }

    ColumnLayout {
        anchors.fill: parent
        anchors.margins: 10
        spacing: 3
        RowLayout {
            Layout.fillWidth: true
            Label { text: wb.compact.step || "step unknown"; color: "white"; font.bold: true; elide: Text.ElideRight; Layout.fillWidth: true }
            Label { text: wb.compact.stepStatus || ""; color: "#fbc02d" }
            ToolButton {
                text: "×"
                palette.buttonText: "white"
                onClicked: compact.standalone ? Qt.quit() : compact.close()
            }
        }
        Label { text: "Blocker: " + (wb.compact.blocker || "none"); color: "#e0e0e0"; elide: Text.ElideRight; Layout.fillWidth: true }
        Label { text: "Open findings " + (wb.compact.findings ?? "?") + "  ·  Codex " + (wb.compact.codex || "unknown"); color: "#e0e0e0" }
        Label { text: "Paid API " + (wb.compact.api || "unknown"); color: "#e0e0e0"; elide: Text.ElideRight; Layout.fillWidth: true
                ToolTip.visible: hover.hovered; ToolTip.text: wb.compact.apiDetail || ""
                HoverHandler { id: hover } }
    }
}

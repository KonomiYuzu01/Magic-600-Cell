import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

// Registered runs (tools/workbench/runs.json of the main checkout) and the latest run records,
// Codex launches included. A run starts only from its Run button, with the registry's fixed steps.
ScrollView {
    id: runsView
    property var statusColors: ({})
    function colorOf(s) { return statusColors[s] || statusColors.unknown || "#6d6d6d" }
    clip: true
    contentWidth: availableWidth

    ColumnLayout {
        width: runsView.availableWidth
        spacing: 6

        Label { text: "Registered runs"; font.bold: true; leftPadding: 8; topPadding: 6 }
        Label {
            visible: wb.registry.problems.length > 0
            text: "The registry is invalid, so nothing can run:\n" + wb.registry.problems.join("\n")
            color: runsView.colorOf("failed"); wrapMode: Text.Wrap; textFormat: Text.PlainText
            Layout.fillWidth: true; leftPadding: 8
        }
        RowLayout {
            Layout.fillWidth: true; Layout.leftMargin: 8; Layout.rightMargin: 8
            Label { text: "Checkout" }
            CheckoutBox { id: runCheckout; objectName: "runCheckout"; Layout.fillWidth: true }
        }
        Repeater {
            model: wb.registry.entries
            delegate: Frame {
                id: entry
                required property var modelData
                Layout.fillWidth: true; Layout.leftMargin: 8; Layout.rightMargin: 8
                ColumnLayout {
                    width: parent.width
                    RowLayout {
                        Layout.fillWidth: true
                        Label { text: entry.modelData.title + " (" + entry.modelData.id + ")"; font.bold: true; elide: Text.ElideRight; Layout.fillWidth: true }
                        Label { text: "limit " + entry.modelData.timeout; opacity: 0.6 }
                        Button {
                            objectName: "run-" + entry.modelData.id
                            text: "Run"
                            enabled: runCheckout.present
                            onClicked: wb.startRun(entry.modelData.id, runCheckout.path, entry.modelData.digest)
                            ToolTip.visible: hovered; ToolTip.text: "Runs exactly the steps shown, with no extra arguments, in the chosen checkout."
                        }
                    }
                    Label { text: entry.modelData.display.join("\n"); font.family: "Consolas"; textFormat: Text.PlainText
                            wrapMode: Text.WrapAnywhere; Layout.fillWidth: true; opacity: 0.8 }
                    Label { visible: entry.modelData.windowsRequired; text: "Windows only"; opacity: 0.6 }
                    Repeater {
                        model: entry.modelData.live
                        delegate: RowLayout {
                            required property var modelData
                            Label { text: "running in " + modelData.checkoutName + " (" + modelData.rid + ")"; color: runsView.colorOf("running") }
                            Button { objectName: "stop-" + modelData.rid; text: "Stop"; onClicked: wb.cancelRun(modelData.rid) }
                            Button { text: "Follow"; onClicked: wb.selectRun(modelData.rid) }
                        }
                    }
                }
            }
        }

        Label { text: "Recent runs and Codex launches"; font.bold: true; leftPadding: 8; topPadding: 10 }
        Repeater {
            model: wb.runs
            delegate: ItemDelegate {
                required property var modelData
                objectName: "runRow-" + modelData.rid
                Layout.fillWidth: true
                highlighted: wb.selected.kind === "run" && wb.selected.id === modelData.rid
                onClicked: wb.selectRun(modelData.rid)
                contentItem: ColumnLayout {
                    RowLayout {
                        Rectangle { width: 10; height: 10; radius: 5; color: runsView.colorOf(modelData.tone) }
                        Label { text: modelData.status.replace("_", " ") + (modelData.stale_heartbeat_s ? " (no heartbeat)" : "")
                                color: runsView.colorOf(modelData.tone); font.bold: true }
                        Label { text: modelData.title || modelData.run; Layout.fillWidth: true; elide: Text.ElideRight }
                        Label { text: modelData.startedText; opacity: 0.6 }
                    }
                    Label {
                        opacity: 0.75; Layout.fillWidth: true; elide: Text.ElideRight; textFormat: Text.PlainText
                        text: modelData.checkoutName
                              + (modelData.exitsText ? " · exits " + modelData.exitsText : "")
                              + (modelData.durationText ? " · " + modelData.durationText : "")
                              + (modelData.call_id ? " · call " + modelData.call_id : "")
                              + (modelData.reasonText ? " · " + modelData.reasonText : "")
                    }
                }
            }
        }
        Label { visible: wb.runs.length === 0; text: "No runs yet."; opacity: 0.6; leftPadding: 8 }
    }
}

import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

// Owner-click launches: a new Claude Code session on a problem packet, or a review-wrapper call
// with allowed values only. Nothing here starts on its own; gate rulings stay in the terminal.
ScrollView {
    id: launchView
    property var packets: []
    // The owner's packet, kept by name when the list reloads; the newest one only until the owner chooses.
    // If it disappears nothing is selected and nothing can start; a new checkout choice starts over.
    property string packetName: ""
    readonly property bool packetChosen: packetBox.currentIndex >= 0 && packetBox.currentText === packetName
    function reload() {
        packets = launchCheckout.present ? wb.packetsFor(launchCheckout.path) : []
        if (packetName === "" && packets.length > 0)
            packetName = packets[0]
        selectPacket()
    }
    function selectPacket() { packetBox.currentIndex = packets.indexOf(packetName) }
    clip: true
    contentWidth: availableWidth
    Component.onCompleted: reload()

    ColumnLayout {
        width: launchView.availableWidth
        spacing: 8

        Label { text: "Every launch needs your click; nothing starts on its own."; opacity: 0.7; leftPadding: 8; topPadding: 6
                wrapMode: Text.Wrap; Layout.fillWidth: true }
        GridLayout {
            columns: 3
            Layout.fillWidth: true; Layout.leftMargin: 8; Layout.rightMargin: 8
            Label { text: "Checkout" }
            CheckoutBox { id: launchCheckout; objectName: "launchCheckout"; Layout.fillWidth: true; Layout.columnSpan: 2
                          onPathChanged: { launchView.packetName = ""; launchView.reload() }
                          onPresentChanged: launchView.reload() }
            Label { text: "Packet" }
            ComboBox { id: packetBox; objectName: "packetBox"; Layout.fillWidth: true; model: launchView.packets
                       displayText: launchView.packetChosen ? currentText : (launchView.packets.length ? "The chosen packet is gone; choose another" : "")
                       onActivated: (index) => { launchView.packetName = textAt(index) }
                       onModelChanged: Qt.callLater(launchView.selectPacket) }
            Button { objectName: "reloadPackets"; text: "Refresh"; onClicked: launchView.reload()
                     ToolTip.visible: hovered; ToolTip.text: "Lists the .md files in work/reviews/packets of the chosen checkout, newest first." }
        }
        Label { visible: launchView.packets.length === 0; text: "No packets in work/reviews/packets of this checkout."; opacity: 0.6; leftPadding: 8 }

        GroupBox {
            title: "New Claude Code session"
            Layout.fillWidth: true; Layout.leftMargin: 8; Layout.rightMargin: 8
            ColumnLayout {
                width: parent.width
                Label { text: "First prompt: Work on the problem packet work/reviews/packets/" + (launchView.packetChosen ? launchView.packetName : "…")
                              + ". Read it first, then follow CLAUDE.md."
                        wrapMode: Text.Wrap; Layout.fillWidth: true; textFormat: Text.PlainText; opacity: 0.8 }
                RowLayout {
                    CheckBox { id: remoteBox; objectName: "remoteBox"; text: "With Remote Control" }
                    Item { Layout.fillWidth: true }
                    Button { objectName: "startSession"; text: "New Claude session"; enabled: launchCheckout.present && launchView.packetChosen
                             onClicked: wb.startSession(launchCheckout.path, launchView.packetName, remoteBox.checked) }
                }
            }
        }

        GroupBox {
            title: "Codex call through the review wrapper"
            Layout.fillWidth: true; Layout.leftMargin: 8; Layout.rightMargin: 8
            ColumnLayout {
                width: parent.width
                GridLayout {
                    columns: 4
                    Label { text: "Kind" }
                    ComboBox { id: kindBox; objectName: "kindBox"; model: wb.codexChoices.kinds }
                    Label { text: "Model" }
                    ComboBox { id: modelBox; objectName: "modelBox"; model: wb.codexChoices.models; implicitContentWidthPolicy: ComboBox.WidestText }
                    Label { text: "Effort" }
                    ComboBox { id: effortBox; objectName: "effortBox"; model: wb.codexChoices.efforts
                               currentIndex: Math.max(0, wb.codexChoices.efforts.indexOf("max")) }
                    Label { text: "Speed" }
                    ComboBox { id: speedBox; objectName: "speedBox"; model: wb.codexChoices.speeds
                               currentIndex: Math.max(0, wb.codexChoices.speeds.indexOf("standard")) }
                }
                Label {
                    text: "python tools/agents/codex_review.py --kind " + kindBox.currentText + " --packet work/reviews/packets/" + (launchView.packetChosen ? launchView.packetName : "…")
                          + " --model " + modelBox.currentText + " --effort " + effortBox.currentText + " --speed " + speedBox.currentText
                    font.family: "Consolas"; wrapMode: Text.WrapAnywhere; Layout.fillWidth: true; textFormat: Text.PlainText; opacity: 0.8
                }
                Label { visible: kindBox.currentText === "implement"; color: "#b26a00"; wrapMode: Text.Wrap; Layout.fillWidth: true
                        text: "No commits, tags or pushes in the repository while an implementation call runs." }
                Label { text: "Runs detached through the workbench runner, with its output in Runs. Gate rulings stay in the terminal."
                        opacity: 0.6; wrapMode: Text.Wrap; Layout.fillWidth: true }
                Button { objectName: "startCodex"; text: "Start Codex call"; enabled: launchCheckout.present && launchView.packetChosen
                         Layout.alignment: Qt.AlignRight
                         onClicked: wb.startCodex(launchCheckout.path, kindBox.currentText, launchView.packetName,
                                                  modelBox.currentText, effortBox.currentText, speedBox.currentText) }
            }
        }
    }
}

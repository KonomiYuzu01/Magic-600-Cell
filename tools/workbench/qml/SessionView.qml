import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

Pane {
    id: view
    property color statusColor: "#6d6d6d"
    readonly property bool isSession: wb.selected.kind === "session"
    property bool showPriority: false
    padding: 10

    Label {
        anchors.centerIn: parent
        visible: !wb.selected.kind
        text: "Select a session, subagent, Codex call or run to follow it."
        opacity: 0.6
    }

    ColumnLayout {
        anchors.fill: parent
        visible: !!wb.selected.kind
        spacing: 8

        RowLayout {
            Layout.fillWidth: true
            Rectangle { width: 12; height: 12; radius: 6; color: view.statusColor }
            Label { text: wb.selected.status || ""; color: view.statusColor; font.bold: true }
            Label { text: wb.selected.title || ""; font.bold: true; font.pixelSize: 16; elide: Text.ElideRight; Layout.fillWidth: true }
            Label { text: wb.selected.detail || ""; opacity: 0.7; elide: Text.ElideRight; Layout.maximumWidth: 360 }
        }

        GridLayout {
            visible: view.isSession && wb.selected.summarySource !== "none"
            columns: 2
            columnSpacing: 10
            Layout.fillWidth: true
            Label { text: "Goal"; opacity: 0.6 }       Label { text: wb.selected.goal || "—"; wrapMode: Text.Wrap; Layout.fillWidth: true }
            Label { text: "Step"; opacity: 0.6 }       Label { text: (wb.selected.step || "—") + (wb.selected.summarySource === "task list" ? "  (from task list)" : ""); wrapMode: Text.Wrap; Layout.fillWidth: true }
            Label { text: "Doing"; opacity: 0.6 }      Label { text: wb.selected.doing || "—"; wrapMode: Text.Wrap; Layout.fillWidth: true }
            Label { text: "Why"; opacity: 0.6 }        Label { text: wb.selected.why || "—"; wrapMode: Text.Wrap; Layout.fillWidth: true }
            Label { text: "Waiting for"; opacity: 0.6 } Label { text: wb.selected.waitingFor || "—"; wrapMode: Text.Wrap; Layout.fillWidth: true }
        }

        Flow {
            Layout.fillWidth: true
            spacing: 6
            Button { visible: view.isSession; text: "Open in Claude Code"; onClicked: wb.openInClaude(false)
                     ToolTip.visible: hovered; ToolTip.text: "Resumes only a session that has ended; otherwise brings Claude Code forward." }
            Button { visible: view.isSession; text: "Resume with Remote Control"; onClicked: wb.openInClaude(true) }
            Button { visible: view.isSession; text: "Git diff"; onClicked: wb.runTool("git-diff", "") }
            Button { objectName: "pauseSession"; visible: view.isSession; text: "Pause after this step"; onClicked: wb.sessionCommand("pause", "") }
            Button { objectName: "resumeSession"; visible: view.isSession; text: "Resume"; onClicked: wb.sessionCommand("resume", "") }
            Button { objectName: "whySession"; visible: view.isSession; text: "Why are you doing this?"; onClicked: wb.sessionCommand("why", "") }
            Button { objectName: "prioritySession"; visible: view.isSession; text: "Priority…"; onClicked: view.showPriority = !view.showPriority }
            TextField { id: priorityText; objectName: "priorityText"; visible: view.isSession && view.showPriority; placeholderText: "What comes first?" }
            Button { objectName: "sendPriority"; visible: view.isSession && view.showPriority; text: "Send priority"; enabled: priorityText.text.trim().length > 0
                     onClicked: if (wb.sessionCommand("priority", priorityText.text)) { priorityText.text = ""; view.showPriority = false } }
            Button { objectName: "stopRun"; visible: wb.selected.kind === "run" && !!wb.selected.cancellable; text: "Stop run"
                     onClicked: wb.cancelRun(wb.selected.id)
                     ToolTip.visible: hovered; ToolTip.text: "Ends the run's whole process tree." }
            Button { text: "Toolchain doctor"; onClicked: wb.runTool("doctor", "") }
            Repeater {
                model: wb.files
                delegate: Button {
                    required property var modelData
                    text: modelData.marimo ? "marimo: " + modelData.label : modelData.label
                    onClicked: modelData.marimo ? wb.runTool("marimo", modelData.path) : wb.openFile(modelData.path)
                    ToolTip.visible: hovered
                    ToolTip.text: modelData.path
                }
            }
        }

        Label {
            visible: view.isSession && (wb.selected.malformed > 0 || wb.selected.malformedLive > 0)
            text: "Skipped malformed lines: " + (wb.selected.malformedLive || wb.selected.malformed)
            color: "#b26a00"
        }

        ListView {
            id: timelineView
            Layout.fillWidth: true
            Layout.fillHeight: true
            clip: true
            model: wb.timeline
            spacing: 4
            ScrollBar.vertical: ScrollBar {}
            property bool follow: true
            onCountChanged: if (follow) Qt.callLater(positionViewAtEnd)
            onMovementEnded: follow = atYEnd
            delegate: Frame {
                required property string time
                required property string kind
                required property string who
                required property string title
                required property string body
                required property string full
                width: ListView.view.width - 12
                padding: 6
                background: Rectangle {
                    radius: 4
                    color: kind === "prompt" ? "#e8f0fe" : kind === "tool" ? "#f3f3f3" : kind === "result" ? "#fafafa" : kind === "event" ? "#fff8e6" : "white"
                    border.color: "#e0e0e0"
                }
                ColumnLayout {
                    width: parent.width
                    RowLayout {
                        Label { text: time; opacity: 0.5; font.family: "Consolas" }
                        Label { text: title; font.bold: true }
                        Label { text: who; opacity: 0.6 }
                        Item { Layout.fillWidth: true }
                        Button { visible: full.length > body.length; text: "Full text"; flat: true
                                 onClicked: { fullDialog.title = title; fullDialog.text = full; fullDialog.open() } }
                    }
                    Label { text: body; wrapMode: Text.Wrap; Layout.fillWidth: true; visible: body.length > 0; textFormat: Text.PlainText
                            font.family: kind === "tool" || kind === "result" ? "Consolas" : "" }
                }
            }
        }

        ColumnLayout {
            visible: view.isSession
            Layout.fillWidth: true
            Label { text: "Owner notes — delivered to Claude at its next tool step; a note cannot grant terminal-only authorizations."; opacity: 0.7; wrapMode: Text.Wrap; Layout.fillWidth: true }
            Repeater {
                model: wb.noteStates
                delegate: Label {
                    required property var modelData
                    text: modelData.id + " — " + (modelData.state === "emitted" ? "sent, not confirmed" : modelData.state) + ": " + modelData.text
                    elide: Text.ElideRight
                    Layout.fillWidth: true
                    color: modelData.state === "answered" ? "#2e7d32" : modelData.state === "emitted" ? "#b26a00" : palette.text
                }
            }
            RowLayout {
                Layout.fillWidth: true
                ScrollView {
                    Layout.fillWidth: true
                    Layout.preferredHeight: 64
                    TextArea { id: noteText; placeholderText: "Note to Claude in this session…"; wrapMode: TextArea.Wrap }
                }
                Button { text: "Send note"; enabled: noteText.text.trim().length > 0
                         onClicked: if (wb.sendNote(noteText.text)) noteText.text = "" }
            }
        }
    }

    Dialog {
        id: fullDialog
        property alias text: fullArea.text
        modal: true
        anchors.centerIn: Overlay.overlay
        width: Math.min(view.width * 0.95, 1100)
        height: Math.min(view.height * 0.9, 800)
        standardButtons: Dialog.Close
        ScrollView {
            anchors.fill: parent
            TextArea { id: fullArea; readOnly: true; wrapMode: TextArea.Wrap; font.family: "Consolas"; selectByMouse: true }
        }
    }
}

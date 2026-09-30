import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

ApplicationWindow {
    id: root
    width: 1360
    height: 860
    visible: true
    title: "Magic 600 Cell workbench"

    readonly property var statusColors: ({ running: "#2e7d32", waiting: "#b26a00", finished: "#546e7a",
                                           failed: "#c62828", unknown: "#6d6d6d" })

    function statusColor(s) { return statusColors[s] || statusColors.unknown }

    Compact {
        id: compactWindow
        visible: false
    }

    header: ToolBar {
        RowLayout {
            anchors.fill: parent
            anchors.leftMargin: 12
            anchors.rightMargin: 8
            spacing: 18
            Label { text: "<b>" + (wb.compact.step || "") + "</b>  " + (wb.compact.stepStatus || ""); textFormat: Text.StyledText }
            Label { text: "Open findings: " + (wb.compact.findings ?? "?") }
            Label { text: "Codex: " + (wb.compact.codex || "unknown") }
            Label { objectName: "attentionCount"; text: "Needs attention: " + wb.flags.length; font.bold: wb.flags.length > 0
                    color: wb.flags.length > 0 ? root.statusColors.failed : palette.windowText }
            Label { text: "Paid API: " + (wb.compact.api || "unknown"); ToolTip.visible: apiHover.hovered; ToolTip.text: wb.compact.apiDetail || ""
                    HoverHandler { id: apiHover } }
            Item { Layout.fillWidth: true }
            Button { text: compactWindow.visible ? "Hide compact view" : "Compact view"; onClicked: compactWindow.visible = !compactWindow.visible }
        }
    }

    SplitView {
        anchors.fill: parent

        ColumnLayout {
            SplitView.preferredWidth: 470
            SplitView.minimumWidth: 320
            spacing: 0

            TabBar {
                id: tabs
                Layout.fillWidth: true
                TabButton { text: "Sessions (" + wb.sessions.length + ")" }
                TabButton { text: "Codex calls (" + wb.calls.length + ")" }
                TabButton { text: "Runs (" + wb.runs.length + ")" }
                TabButton { text: "Launch" }
                TabButton { text: "Progress" }
            }

            StackLayout {
                currentIndex: tabs.currentIndex
                Layout.fillWidth: true
                Layout.fillHeight: true

                ListView {
                    id: sessionList
                    clip: true
                    model: wb.sessions
                    spacing: 2
                    ScrollBar.vertical: ScrollBar {}
                    header: ColumnLayout {
                        width: ListView.view.width
                        spacing: 2
                        visible: wb.flags.length > 0
                        height: visible ? implicitHeight + 6 : 0
                        Label { text: "Needs attention (" + wb.flags.length + ")"; font.bold: true; leftPadding: 8; topPadding: 6
                                color: root.statusColors.failed }
                        Repeater {
                            model: wb.flags
                            delegate: ItemDelegate {
                                required property var modelData
                                objectName: "flag-" + modelData.kind + "-" + modelData.target_id
                                Layout.fillWidth: true
                                onClicked: wb.selectFlag(modelData.target_kind, modelData.target_id)
                                contentItem: RowLayout {
                                    Rectangle { width: 8; height: 8; radius: 4
                                                color: modelData.severity === 0 ? root.statusColors.waiting
                                                       : modelData.severity === 1 ? root.statusColors.failed : root.statusColors.unknown }
                                    Label { text: modelData.kind.replace("_", " "); font.bold: true }
                                    Label { text: modelData.text; elide: Text.ElideRight; Layout.fillWidth: true; textFormat: Text.PlainText }
                                    Label { text: modelData.age; opacity: 0.6 }
                                }
                            }
                        }
                    }
                    delegate: ItemDelegate {
                        id: sessionItem
                        required property var modelData
                        width: ListView.view.width
                        highlighted: wb.selected.kind === "session" && wb.selected.id === modelData.sid
                        onClicked: wb.selectSession(modelData.sid)
                        contentItem: ColumnLayout {
                            spacing: 3
                            RowLayout {
                                Rectangle { width: 10; height: 10; radius: 5; color: root.statusColor(modelData.status) }
                                Label { text: modelData.status; color: root.statusColor(modelData.status); font.bold: true }
                                Label { text: modelData.title; elide: Text.ElideRight; Layout.fillWidth: true; font.bold: true }
                                Label { text: modelData.last; opacity: 0.6 }
                            }
                            Label { text: modelData.checkout + (modelData.branch ? " · " + modelData.branch : "") + (modelData.detail ? " · " + modelData.detail : "")
                                    opacity: 0.7; elide: Text.ElideRight; Layout.fillWidth: true }
                            Label { visible: modelData.goal !== ""; text: "Goal: " + modelData.goal; wrapMode: Text.Wrap; Layout.fillWidth: true }
                            Label { visible: modelData.step !== ""; text: "Step: " + modelData.step + (modelData.summarySource === "task list" ? "  (from task list)" : "")
                                    wrapMode: Text.Wrap; Layout.fillWidth: true }
                            Label { visible: modelData.doing !== ""; text: "Doing: " + modelData.doing + (modelData.why ? " — because " + modelData.why : "")
                                    wrapMode: Text.Wrap; Layout.fillWidth: true }
                            Label { visible: modelData.waitingFor !== ""; text: "Waiting for: " + modelData.waitingFor; wrapMode: Text.Wrap; Layout.fillWidth: true }
                            Label { visible: modelData.summarySource === "none"; text: "No brief or task list yet."; opacity: 0.6 }
                            Label { visible: modelData.capped; text: "Event file capped; status comes from the transcript."; color: "#b26a00" }
                            Repeater {
                                model: modelData.subagents
                                delegate: Label {
                                    required property var modelData
                                    text: "  ↳ subagent " + modelData.id + (modelData.type ? " (" + modelData.type + ")" : "") + ": " + modelData.status
                                    color: root.statusColor(modelData.status)
                                    TapHandler { onTapped: wb.selectSubagent(sessionItem.modelData.sid, modelData.id) }
                                }
                            }
                            Repeater {
                                model: modelData.calls
                                delegate: Label {
                                    required property var modelData
                                    text: "  ↳ Codex " + modelData.kind + " " + modelData.call_id + ": " + modelData.status
                                    color: root.statusColor(modelData.status)
                                    TapHandler { onTapped: wb.selectCall(modelData.call_id) }
                                }
                            }
                        }
                    }
                    Label { anchors.centerIn: parent; visible: sessionList.count === 0; text: "No sessions of this repository found."; opacity: 0.6 }
                }

                ListView {
                    clip: true
                    model: wb.calls
                    spacing: 2
                    ScrollBar.vertical: ScrollBar {}
                    delegate: ItemDelegate {
                        required property var modelData
                        width: ListView.view.width
                        highlighted: wb.selected.kind === "call" && wb.selected.id === modelData.call_id
                        onClicked: wb.selectCall(modelData.call_id)
                        contentItem: ColumnLayout {
                            RowLayout {
                                Rectangle { width: 10; height: 10; radius: 5; color: root.statusColor(modelData.status) }
                                Label { text: modelData.status; color: root.statusColor(modelData.status); font.bold: true }
                                Label { text: (modelData.kind || "call") + " " + modelData.call_id; Layout.fillWidth: true; elide: Text.ElideRight }
                                Label { text: modelData.started; opacity: 0.6 }
                            }
                            Label {
                                opacity: 0.75; Layout.fillWidth: true; wrapMode: Text.Wrap
                                text: modelData.checkoutName
                                      + (modelData.verdict ? " · verdict " + modelData.verdict : "")
                                      + (modelData.findings !== null && modelData.findings !== undefined ? " · " + modelData.findings + " finding(s)" : "")
                                      + (modelData.blockingText ? " · blocking " + modelData.blockingText : "")
                                      + (modelData.session ? " · session " + modelData.session.slice(0, 8) + " (" + modelData.link + ")" : "")
                            }
                        }
                    }
                }

                RunsView { statusColors: root.statusColors }

                LaunchView {}

                ListView {
                    clip: true
                    model: wb.progress.steps || []
                    spacing: 2
                    ScrollBar.vertical: ScrollBar {}
                    header: Label { padding: 8; text: "Public progress (docs/progress/status.json), updated " + (wb.progress.updated || "?"); opacity: 0.7 }
                    delegate: Frame {
                        required property var modelData
                        width: ListView.view.width
                        background: Rectangle { color: modelData.id === wb.progress.current ? "#fff4d6" : "transparent"; border.color: "#dddddd" }
                        ColumnLayout {
                            width: parent.width
                            Label { text: "<b>" + modelData.id + "</b> " + modelData.title + " — " + modelData.status.replace("_", " "); textFormat: Text.StyledText
                                    wrapMode: Text.Wrap; Layout.fillWidth: true }
                            Label { text: "Acceptance: " + (modelData.acceptance || "not defined"); wrapMode: Text.Wrap; Layout.fillWidth: true; opacity: 0.8 }
                            Label { visible: !!modelData.blocker; text: "Blocker: " + (modelData.blocker || ""); color: "#c62828"; wrapMode: Text.Wrap; Layout.fillWidth: true }
                        }
                    }
                }
            }
        }

        SessionView {
            SplitView.fillWidth: true
            statusColor: root.statusColor(wb.selected.status || "unknown")
        }
    }

    footer: Label {
        padding: 6
        text: wb.message || "Reads local sessions in place; writes only owner notes and run records; starts runs, Codex calls and sessions only on your click."
        elide: Text.ElideRight
        opacity: wb.message ? 1 : 0.6
    }
}

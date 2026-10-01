import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

Popup {
    id: preview
    objectName: "galleryPreview"
    property var entry: ({ path: "", rel: "", preview: "", source: "", openHow: "", sourceHow: "", canMarimo: false, sessions: [],
                           sessionTitles: [] })
    property string target: ""          // the session that receives star, wrong-direction and comment notes
    readonly property bool hasSession: target !== ""
    parent: Overlay.overlay
    anchors.centerIn: parent
    width: Math.min(1000, parent.width - 32)
    height: Math.min(740, parent.height - 32)
    modal: true
    padding: 12

    // `preferredSid` (the gallery filter, or the asking session) gets the notes when it is one of the
    // file's sessions; otherwise the first listed. With several sessions the owner can choose another.
    function showItem(row, preferredSid) {
        entry = row
        target = preferredSid && row.sessions.indexOf(preferredSid) >= 0 ? preferredSid : (row.sessions.length > 0 ? row.sessions[0] : "")
        recipient.currentIndex = Math.max(0, row.sessions.indexOf(target))
        comment.text = ""
        open()
    }

    contentItem: ColumnLayout {
        spacing: 8
        RowLayout {
            Layout.fillWidth: true
            Label { text: preview.entry.rel; font.bold: true; elide: Text.ElideRight; Layout.fillWidth: true; textFormat: Text.PlainText }
            Button { objectName: "galleryClose"; text: "Close"; onClicked: preview.close() }
        }
        Item {
            Layout.fillWidth: true
            Layout.fillHeight: true
            Image {
                anchors.fill: parent
                asynchronous: true
                cache: false
                sourceSize: Qt.size(Math.max(1, Math.floor(width)), Math.max(1, Math.floor(height)))
                fillMode: Image.PreserveAspectFit
                source: preview.entry.preview ? Qt.url("file:///" + preview.entry.preview.replace(/\\/g, "/").replace(/%/g, "%25").replace(/#/g, "%23").replace(/\?/g, "%3F")) : ""
            }
            Label { anchors.centerIn: parent; visible: !preview.entry.preview; text: "No in-app preview. Open externally to view this file."; opacity: 0.7 }
        }
        Flow {
            Layout.fillWidth: true
            spacing: 6
            Button {
                objectName: "galleryOpen"
                visible: preview.entry.openHow !== ""
                text: preview.entry.openHow === "browser" ? "Open in browser" : preview.entry.openHow === "notepad" ? "Open as text" : "Open"
                onClicked: wb.openFile(preview.entry.path)
            }
            Button { objectName: "galleryOpenSource"; visible: preview.entry.sourceHow !== ""; text: "Open source"; onClicked: wb.openFile(preview.entry.source) }
            Button { objectName: "galleryMarimo"; visible: preview.entry.canMarimo; text: "marimo"; onClicked: wb.runTool("marimo", preview.entry.path) }
        }
        RowLayout {
            Layout.fillWidth: true
            visible: preview.entry.sessions.length > 1
            Label { text: "Notes go to" }
            ComboBox {
                id: recipient
                objectName: "galleryRecipient"
                Layout.fillWidth: true
                textRole: "title"
                valueRole: "sid"
                model: preview.entry.sessions.map(function(sid, i) {
                    return { sid: sid, title: (preview.entry.sessionTitles || [])[i] || sid.slice(0, 8) }
                })
                onActivated: (index) => { preview.target = valueAt(index) }
            }
        }
        RowLayout {
            Layout.fillWidth: true
            Button { objectName: "galleryStar"; text: "Star"; enabled: preview.hasSession; onClicked: wb.fileNote(preview.entry.path, "star", "", preview.target)
                     ToolTip.visible: hovered; ToolTip.text: preview.hasSession ? "" : "No session produced this file" }
            Button { objectName: "galleryWrong"; text: "Wrong direction"; enabled: preview.hasSession; onClicked: wb.fileNote(preview.entry.path, "wrong", comment.text, preview.target)
                     ToolTip.visible: hovered; ToolTip.text: preview.hasSession ? "" : "No session produced this file" }
            TextField { id: comment; objectName: "galleryComment"; Layout.fillWidth: true; placeholderText: "Comment on this file"; enabled: preview.hasSession }
            Button { objectName: "galleryCommentSend"; text: "Send"; enabled: preview.hasSession && comment.text.trim().length > 0
                     onClicked: if (wb.fileNote(preview.entry.path, "comment", comment.text, preview.target)) comment.text = ""
                     ToolTip.visible: hovered; ToolTip.text: preview.hasSession ? "" : "No session produced this file" }
            HoverHandler { id: noteHover }
            ToolTip.visible: !preview.hasSession && noteHover.hovered
            ToolTip.text: "No session produced this file"
        }
    }
}

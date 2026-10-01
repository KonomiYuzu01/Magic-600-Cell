import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

Pane {
    id: homeView
    padding: 10
    property string focusedCard: ""
    property var drafts: ({})          // card id -> unsent reply; survives inbox model changes
    property string gallerySid: ""     // the gallery session filter, kept by sid when the session list changes
    readonly property var filteredGallery: wb.gallery.filter(function(row) {
        return (galleryType.currentIndex <= 0 || row.type === galleryType.currentText)
            && (gallerySid === "" || row.sessions.indexOf(gallerySid) >= 0)
    })

    function positionCard(id) {
        focusedCard = id
        for (var i = 0; i < wb.inbox.length; ++i) {
            if (wb.inbox[i].id === id) {
                inboxList.currentIndex = i
                Qt.callLater(function() { inboxList.positionViewAtIndex(inboxList.currentIndex, ListView.Contain) })
                return
            }
        }
        answeredToggle.checked = true
        Qt.callLater(inboxList.positionViewAtEnd)
    }

    function showAttachment(card, attachment) {
        for (var i = 0; i < wb.gallery.length; ++i) {
            if (wb.gallery[i].path === attachment.path) {
                galleryPreview.showItem(wb.gallery[i], card.sid)
                return
            }
        }
        galleryPreview.showItem({ path: attachment.path, rel: attachment.rel, source: attachment.path,
                                  preview: attachment.previewable ? attachment.path : "", openHow: "", sourceHow: "",
                                  canMarimo: false, sessions: card.sid ? [card.sid] : [],
                                  sessionTitles: card.sid ? [card.sessionTitle] : [] }, card.sid)
    }

    GalleryPreview { id: galleryPreview }

    ColumnLayout {
        anchors.fill: parent
        spacing: 10
        RowLayout {
            id: progressBand
            objectName: "progressBand"
            Layout.fillWidth: true
            spacing: 10
            Repeater {
                model: wb.homeProgress.tracks
                delegate: Frame {
                    id: trackView
                    required property var modelData
                    property bool expanded: false
                    readonly property var current: modelData.current
                    readonly property var remaining: current ? current.remaining : []
                    readonly property bool counted: !wb.homeProgress.noChecklist && modelData.percent !== null
                    Layout.fillWidth: true
                    Layout.preferredWidth: 1
                    Layout.alignment: Qt.AlignTop
                    padding: 10
                    background: Rectangle { color: palette.base; border.color: palette.mid; radius: 6 }
                    ColumnLayout {
                        width: parent.width
                        spacing: 4
                        RowLayout {
                            Layout.fillWidth: true
                            Label { text: trackView.modelData.title; font.bold: true; font.pixelSize: 15; textFormat: Text.PlainText }
                            Item { Layout.fillWidth: true }
                            Label { objectName: "percent-" + trackView.modelData.id; font.bold: true
                                    text: trackView.counted ? Math.floor(trackView.modelData.percent) + "%" : "no checklist" }
                        }
                        ProgressBar { objectName: "track-" + trackView.modelData.id; Layout.fillWidth: true; visible: trackView.counted
                                      from: 0; to: 100; value: trackView.modelData.percent ?? 0 }
                        RowLayout {
                            visible: !!trackView.current
                            Layout.fillWidth: true
                            Layout.topMargin: 4
                            Label { text: trackView.current ? "Now: " + trackView.current.title : ""; elide: Text.ElideRight
                                    Layout.fillWidth: true; textFormat: Text.PlainText }
                            Label { objectName: "currentPercent-" + trackView.modelData.id; opacity: 0.8
                                    visible: trackView.counted && !!trackView.current && trackView.current.percent !== null
                                    text: trackView.current && trackView.current.percent !== null ? Math.floor(trackView.current.percent) + "%" : "" }
                        }
                        ProgressBar { objectName: "current-" + trackView.modelData.id; Layout.fillWidth: true
                                      visible: trackView.counted && !!trackView.current && trackView.current.percent !== null
                                      from: 0; to: 100; value: trackView.current ? (trackView.current.percent ?? 0) : 0 }
                        Repeater {
                            model: trackView.expanded ? trackView.remaining : trackView.remaining.slice(0, 3)
                            delegate: Label {
                                required property var modelData
                                Layout.fillWidth: true
                                elide: Text.ElideRight
                                font.pixelSize: 12
                                textFormat: Text.PlainText
                                text: "\u25cb " + modelData.title + (modelData.weight > 1 ? "  (weight " + modelData.weight + ")" : "")
                            }
                        }
                        Label {
                            objectName: "moreItems-" + trackView.modelData.id
                            visible: trackView.remaining.length > 3
                            text: trackView.expanded ? "Show fewer" : "+" + (trackView.remaining.length - 3) + " more items left"
                            color: "#3569a8"
                            font.pixelSize: 12
                            TapHandler { onTapped: trackView.expanded = !trackView.expanded }
                        }
                    }
                }
            }
            Frame {
                visible: wb.homeProgress.sessions.length > 0
                Layout.alignment: Qt.AlignTop
                Layout.preferredWidth: 280
                padding: 10
                background: Rectangle { color: palette.base; border.color: palette.mid; radius: 6 }
                ColumnLayout {
                    width: parent.width
                    spacing: 4
                    Label { text: "Session tasks"; font.bold: true; font.pixelSize: 15 }
                    Repeater {
                        model: wb.homeProgress.sessions
                        delegate: RowLayout {
                            required property var modelData
                            Layout.fillWidth: true
                            Label { text: modelData.title; elide: Text.ElideRight; Layout.fillWidth: true; textFormat: Text.PlainText }
                            Label { objectName: "tasks-" + modelData.sid; text: modelData.done + "/" + modelData.total; font.bold: true }
                        }
                    }
                }
            }
            Label { visible: wb.homeProgress.tracks.length === 0; text: wb.homeProgress.noChecklist ? "no checklist" : "step unknown"; opacity: 0.7 }
        }

        SplitView {
            Layout.fillWidth: true
            Layout.fillHeight: true
            orientation: Qt.Horizontal
            ColumnLayout {
                SplitView.preferredWidth: 510
                SplitView.minimumWidth: 300
                Label { text: "For you (" + wb.home.forYou + ")"; font.bold: true; font.pixelSize: 16 }
                ListView {
                    id: inboxList
                    objectName: "inboxList"
                    Layout.fillWidth: true
                    Layout.fillHeight: true
                    model: wb.inbox
                    spacing: 8
                    clip: true
                    ScrollBar.vertical: ScrollBar {}
                    delegate: InboxCard {
                        required property var modelData
                        width: ListView.view.width
                        card: modelData
                        draft: homeView.drafts[modelData.id] || ""
                        onPreviewRequested: (attachment) => homeView.showAttachment(card, attachment)
                        onDraftEdited: (text) => {
                            if (text)
                                homeView.drafts[card.id] = text
                            else
                                delete homeView.drafts[card.id]
                        }
                    }
                    header: Label { width: ListView.view.width; padding: 8; visible: inboxList.count === 0; height: visible ? implicitHeight : 0; text: "Nothing needs you right now."; opacity: 0.7 }
                    footer: ColumnLayout {
                        width: ListView.view.width
                        Button { id: answeredToggle; objectName: "answeredToggle"; text: "Answered (" + wb.answered.length + ")"; checkable: true; checked: false }
                        ColumnLayout {
                            visible: answeredToggle.checked
                            Layout.fillWidth: true
                            Repeater {
                                model: wb.answered
                                delegate: Frame {
                                    required property var modelData
                                    objectName: "answered-" + modelData.id
                                    Layout.fillWidth: true
                                    ColumnLayout {
                                        width: parent.width
                                        Label { text: modelData.title; font.bold: true; wrapMode: Text.Wrap; Layout.fillWidth: true; textFormat: Text.PlainText }
                                        Label { text: modelData.answer; wrapMode: Text.Wrap; Layout.fillWidth: true; textFormat: Text.PlainText }
                                        Label { text: modelData.state === "acknowledged" ? "acknowledged by Claude" : "sent, delivery not confirmed"; opacity: 0.7 }
                                    }
                                }
                            }
                        }
                    }
                }
            }
            ColumnLayout {
                SplitView.fillWidth: true
                SplitView.minimumWidth: 360
                Label { text: "Gallery"; font.bold: true; font.pixelSize: 16 }
                RowLayout {
                    Layout.fillWidth: true
                    ComboBox { id: galleryType; objectName: "galleryType"; model: ["All types", "image", "vector", "pdf", "video", "diagram", "slides", "notebook"] }
                    ComboBox {
                        id: gallerySession; objectName: "gallerySession"; Layout.fillWidth: true; textRole: "title"; valueRole: "sid"
                        model: [{ sid: "", title: "All sessions" }].concat(wb.gallerySessions)
                        function sync() {
                            var i = indexOfValue(homeView.gallerySid)
                            if (i < 0)
                                homeView.gallerySid = ""   // the chosen session is no longer listed
                            currentIndex = Math.max(0, i)
                        }
                        onActivated: (index) => { homeView.gallerySid = valueAt(index) }
                        onModelChanged: Qt.callLater(gallerySession.sync)
                    }
                    Button { objectName: "galleryRescan"; text: "Rescan"; onClicked: wb.rescanGallery() }
                }
                GridView {
                    id: galleryGrid
                    objectName: "galleryGrid"
                    Layout.fillWidth: true
                    Layout.fillHeight: true
                    cellWidth: Math.max(160, width / Math.max(1, Math.floor(width / 220)))
                    cellHeight: 180
                    model: homeView.filteredGallery
                    cacheBuffer: 0
                    clip: true
                    ScrollBar.vertical: ScrollBar {}
                    delegate: ItemDelegate {
                        id: tile
                        required property var modelData
                        objectName: "galleryTile-" + modelData.rel.split("/").pop()
                        width: galleryGrid.cellWidth - 8
                        height: galleryGrid.cellHeight - 8
                        onClicked: galleryPreview.showItem(modelData, homeView.gallerySid)
                        contentItem: ColumnLayout {
                            Item {
                                Layout.fillWidth: true
                                Layout.fillHeight: true
                                Image {
                                    anchors.fill: parent
                                    asynchronous: true
                                    cache: false
                                    sourceSize: Qt.size(320, 200)
                                    fillMode: Image.PreserveAspectFit
                                    source: tile.modelData.preview ? Qt.url("file:///" + tile.modelData.preview.replace(/\\/g, "/").replace(/%/g, "%25").replace(/#/g, "%23").replace(/\?/g, "%3F")) : ""
                                }
                                Rectangle {
                                    anchors.fill: parent
                                    visible: !tile.modelData.preview
                                    color: "#e8edf3"
                                    radius: 4
                                }
                                Label {
                                    objectName: "placeholder-" + tile.modelData.rel.split("/").pop()
                                    anchors.centerIn: parent
                                    width: parent.width - 12
                                    visible: !tile.modelData.preview
                                    horizontalAlignment: Text.AlignHCenter
                                    wrapMode: Text.Wrap
                                    color: "#44505e"
                                    text: {
                                        var m = tile.modelData
                                        var head = m.type.toUpperCase() + "  " + m.ext
                                        if (m.pixels.length)
                                            return head + "\n" + m.pixels[0] + "×" + m.pixels[1] + ", too large to preview"
                                        if (m.type === "pdf" || m.type === "video")
                                            return head + "\nopens in its own app"
                                        if (m.type === "notebook")
                                            return head + "\nopens in marimo"
                                        return head + "\nno rendered image yet"
                                    }
                                }
                            }
                            RowLayout {
                                Layout.fillWidth: true
                                Label { text: tile.modelData.rel.split("/").pop(); elide: Text.ElideRight; Layout.fillWidth: true; textFormat: Text.PlainText }
                                Label { visible: tile.modelData.isNew; text: "new"; color: "#3569a8"; font.pixelSize: 12 }
                            }
                        }
                    }
                }
            }
        }
    }
}

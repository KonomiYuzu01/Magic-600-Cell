import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

ApplicationWindow {
    id: window
    objectName: "rateWindow"
    width: 1100
    height: 800
    minimumWidth: 600
    minimumHeight: 420
    visible: true
    title: "Taste Lab"
    color: "#202124"

    FocusScope {
        id: ratingFocus
        objectName: "ratingFocus"
        anchors.fill: parent
        focus: true
        Keys.onPressed: function(event) {
            if (event.isAutoRepeat && (event.key === Qt.Key_Delete || event.key === Qt.Key_Left ||
                                       event.key === Qt.Key_Right || event.key === Qt.Key_Down ||
                                       event.key === Qt.Key_Backspace)) {
                event.accepted = true
                return
            }
            event.accepted = rate.key(event.key, event.modifiers, event.isAutoRepeat)
        }

        ColumnLayout {
            anchors.fill: parent
            anchors.margins: 16
            spacing: 10

            Label {
                objectName: "header"
                Layout.fillWidth: true
                text: rate.headerText
                textFormat: Text.PlainText
                color: "#eeeeee"
                wrapMode: Text.WordWrap
                font.pixelSize: 16
            }

            Item {
                id: imagePane
                Layout.fillWidth: true
                Layout.fillHeight: true

                Image {
                    id: mainImage
                    objectName: "mainImage"
                    property bool admitted: false
                    anchors.fill: parent
                    source: rate.image
                    asynchronous: true
                    cache: true
                    visible: admitted
                    onSourceChanged: admitted = false
                    onStatusChanged: if (status === Image.Ready) admitted = rate.imageLoaded(String(source))
                    fillMode: Image.PreserveAspectFit
                    sourceSize.width: Math.max(1, Math.ceil(imagePane.width))
                    sourceSize.height: Math.max(1, Math.ceil(imagePane.height))
                }

                Repeater {
                    model: rate.preload
                    delegate: Image {
                        required property string modelData
                        required property int index
                        objectName: "preload-" + index
                        source: modelData
                        asynchronous: true
                        cache: true
                        onStatusChanged: if (status === Image.Ready) rate.imageLoaded(String(source))
                        visible: false
                        sourceSize.width: mainImage.sourceSize.width
                        sourceSize.height: mainImage.sourceSize.height
                    }
                }

                Label {
                    objectName: "emptyState"
                    anchors.centerIn: parent
                    width: Math.min(parent.width, 560)
                    visible: rate.empty
                    text: "No images to rate. Run fetch.py and embed.py, or the tastelab-fetch run in the workbench."
                    textFormat: Text.PlainText
                    wrapMode: Text.WordWrap
                    horizontalAlignment: Text.AlignHCenter
                    color: "#dddddd"
                }
            }

            Label {
                objectName: "caption"
                Layout.fillWidth: true
                text: rate.caption
                textFormat: Text.PlainText
                color: "#aaaaaa"
                font.pixelSize: 12
                elide: Text.ElideRight
            }

            TextField {
                id: noteField
                objectName: "noteField"
                Layout.fillWidth: true
                visible: rate.noteEditing
                maximumLength: 300
                text: rate.note
                placeholderText: "One-line note · Enter keeps it, Escape drops it"
                onTextEdited: rate.setNote(text)
                Keys.onPressed: function(event) {
                    if (event.key === Qt.Key_Return || event.key === Qt.Key_Enter || event.key === Qt.Key_Escape) {
                        rate.finishNote(event.key !== Qt.Key_Escape)
                        event.accepted = true
                    }
                }
            }

            Label {
                objectName: "statusLine"
                Layout.fillWidth: true
                text: rate.message
                textFormat: Text.PlainText
                color: rate.removeArmed ? "#ffca80" : "#dddddd"
                elide: Text.ElideRight
            }

            Label {
                Layout.fillWidth: true
                text: "Right like, Left dislike, Down skip, N note, Backspace undo · Delete twice removes"
                textFormat: Text.PlainText
                color: "#aaaaaa"
                font.pixelSize: 12
                wrapMode: Text.WordWrap
            }
        }

    }

    Popup {
        id: proposalOverlay
        objectName: "proposalOverlay"
        parent: window.contentItem
        anchors.centerIn: parent
        width: Math.min(660, window.width - 40)
        height: Math.min(implicitHeight, window.height - 40)
        padding: 20
        modal: true
        focus: true
        closePolicy: Popup.NoAutoClose
        visible: rate.proposals.length > 0
        onClosed: if (!rate.noteEditing) ratingFocus.forceActiveFocus()
        contentItem: ColumnLayout {
            spacing: 10
            Label {
                text: "Try these search terms?"
                font.pixelSize: 18
            }
            Label {
                text: "Only ticked terms will be fetched."
                Layout.fillWidth: true
                wrapMode: Text.WordWrap
            }
            ScrollView {
                Layout.fillWidth: true
                Layout.fillHeight: true
                implicitHeight: terms.implicitHeight
                clip: true
                ColumnLayout {
                    id: terms
                    width: proposalOverlay.availableWidth
                    Repeater {
                        model: rate.proposals
                        delegate: CheckBox {
                            id: proposalChoice
                            required property var modelData
                            required property int index
                            objectName: "proposal-" + index
                            Layout.fillWidth: true
                            text: modelData.term + " · " + (modelData.category || "probe") + " · " + modelData.origin
                            checked: false
                            onToggled: rate.checkProposal(modelData.term, checked)
                            contentItem: Label {
                                text: proposalChoice.text
                                textFormat: Text.PlainText
                                font: proposalChoice.font
                                leftPadding: proposalChoice.indicator.width + proposalChoice.spacing
                                verticalAlignment: Text.AlignVCenter
                                elide: Text.ElideRight
                            }
                        }
                    }
                }
            }
            RowLayout {
                Button {
                    objectName: "fetchProposals"
                    text: "Fetch ticked terms"
                    onClicked: rate.acceptProposals()
                }
                Button {
                    objectName: "rejectProposals"
                    text: "Not now"
                    onClicked: rate.rejectProposals()
                }
            }
        }
    }

    Connections {
        target: rate
        function onNoteChanged() {
            if (rate.noteEditing && rate.proposals.length === 0)
                noteField.forceActiveFocus()
            else if (rate.proposals.length === 0) {
                noteField.focus = false
                ratingFocus.forceActiveFocus()
            }
        }
        function onProposalsChanged() {
            if (rate.proposals.length === 0) {
                if (rate.noteEditing) noteField.forceActiveFocus()
                else ratingFocus.forceActiveFocus()
            }
        }
    }

    Component.onCompleted: ratingFocus.forceActiveFocus()
}

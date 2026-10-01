import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

Frame {
    id: cardView
    required property var card
    property string draft: ""          // an unsent reply, kept by HomeView while the inbox model changes
    readonly property bool answerable: card.state === "open"
    signal previewRequested(var attachment)
    signal draftEdited(string text)
    objectName: "card-" + card.id
    padding: 10

    function fileUrl(path) {
        return Qt.url("file:///" + path.replace(/\\/g, "/").replace(/%/g, "%25").replace(/#/g, "%23").replace(/\?/g, "%3F"))
    }

    ColumnLayout {
        width: parent.width
        spacing: 6
        RowLayout {
            Layout.fillWidth: true
            Label { text: cardView.card.title; font.bold: true; wrapMode: Text.Wrap; Layout.fillWidth: true; textFormat: Text.PlainText }
            Label { visible: cardView.card.blocking; text: "Blocking"; color: "#c62828"; font.bold: true }
        }
        Label { text: cardView.card.sessionTitle + " · " + cardView.card.age; opacity: 0.7; wrapMode: Text.Wrap; Layout.fillWidth: true; textFormat: Text.PlainText }
        Label { visible: text.length > 0; text: cardView.card.context; wrapMode: Text.Wrap; Layout.fillWidth: true; textFormat: Text.PlainText }
        Label { visible: text.length > 0; text: cardView.card.detail; wrapMode: Text.Wrap; Layout.fillWidth: true; textFormat: Text.PlainText }

        Flickable {
            Layout.fillWidth: true
            Layout.preferredHeight: attachmentRow.implicitHeight
            visible: cardView.card.attachments.length > 0
            contentWidth: attachmentRow.implicitWidth
            contentHeight: attachmentRow.implicitHeight
            clip: true
            RowLayout {
                id: attachmentRow
                spacing: 8
                Repeater {
                    model: cardView.card.attachments
                    delegate: ColumnLayout {
                        required property var modelData
                        required property int index
                        Image {
                            Layout.preferredWidth: cardView.card.askKind === "pick" ? 240 : 160
                            Layout.preferredHeight: cardView.card.askKind === "pick" ? 160 : 100
                            asynchronous: true
                            cache: false
                            sourceSize: cardView.card.askKind === "pick" ? Qt.size(480, 320) : Qt.size(320, 200)
                            fillMode: Image.PreserveAspectFit
                            source: modelData.previewable ? cardView.fileUrl(modelData.path) : ""
                            TapHandler { onTapped: cardView.previewRequested(modelData) }
                        }
                        Label { text: modelData.rel; elide: Text.ElideRight; Layout.maximumWidth: 240; textFormat: Text.PlainText }
                        Button {
                            objectName: "pick-" + cardView.card.id + "-" + index
                            visible: cardView.card.askKind === "pick"
                            enabled: cardView.answerable
                            text: "Pick"
                            onClicked: wb.answerCard(cardView.card.id, cardView.card.options[index])
                        }
                    }
                }
            }
        }

        Flow {
            visible: cardView.card.askKind !== "pick"
            Layout.fillWidth: true
            spacing: 6
            Repeater {
                model: cardView.card.options
                delegate: Button {
                    required property string modelData
                    required property int index
                    objectName: "option-" + cardView.card.id + "-" + index
                    enabled: cardView.answerable
                    text: modelData
                    onClicked: wb.answerCard(cardView.card.id, modelData)
                }
            }
        }
        RowLayout {
            visible: cardView.card.askKind !== "pick" && cardView.card.answerRef !== ""
            Layout.fillWidth: true
            TextField { id: reply; objectName: "reply-" + cardView.card.id; Layout.fillWidth: true; placeholderText: "Your reply"
                        enabled: cardView.answerable
                        onTextEdited: cardView.draftEdited(text)
                        Component.onCompleted: text = cardView.draft }
            Button { objectName: "send-" + cardView.card.id; text: "Send"; enabled: cardView.answerable && reply.text.trim().length > 0
                     onClicked: {
                         var text = reply.text
                         cardView.draftEdited("")   // before answerCard: a successful answer rebuilds the inbox
                         if (wb.answerCard(cardView.card.id, text))
                             reply.text = ""
                         else
                             cardView.draftEdited(text)
                     } }
        }
        Flow {
            Layout.fillWidth: true
            spacing: 6
            Button { objectName: "open-" + cardView.card.id; visible: cardView.card.canOpenSession; text: "Open in Claude Code"
                     onClicked: wb.openCardSession(cardView.card.id) }
            Repeater {
                model: cardView.card.files
                delegate: Button {
                    required property string modelData
                    text: modelData.replace(/\\/g, "/").split("/").pop()
                    onClicked: wb.openFile(modelData)
                }
            }
        }
        Label {
            objectName: "queued-" + cardView.card.id
            visible: cardView.card.state === "queued"
            text: "Queued: Claude gets this at its next tool step"
                  + (cardView.card.needsOwner ? ". The session is idle: open it in Claude Code and send any message." : "")
            color: "#b26a00"; wrapMode: Text.Wrap; Layout.fillWidth: true; textFormat: Text.PlainText
        }
    }
}

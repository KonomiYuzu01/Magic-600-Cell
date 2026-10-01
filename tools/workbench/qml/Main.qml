import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

ApplicationWindow {
    id: root
    objectName: "mainWindow"
    width: 1360
    height: 860
    visible: true
    title: "Magic 600 Cell workbench"

    Compact { id: compactWindow; visible: false }

    header: ToolBar {
        RowLayout {
            anchors.fill: parent
            anchors.leftMargin: 12
            anchors.rightMargin: 8
            spacing: 18
            Label { objectName: "headerStep"; text: wb.home.stepText; font.bold: true }
            Label {
                objectName: "headerForYou"
                text: "For you " + wb.home.forYou
                font.bold: wb.home.forYou > 0
                color: wb.home.forYou > 0 ? "#3569a8" : palette.windowText
            }
            Label { objectName: "headerGallery"; text: "Gallery +" + wb.home.galleryNew }
            Label {
                objectName: "headerApi"
                visible: wb.home.apiVisible
                text: "Paid API: " + wb.home.apiText
                ToolTip.visible: apiHover.hovered; ToolTip.text: wb.compact.apiDetail || ""
                HoverHandler { id: apiHover }
            }
            Item { Layout.fillWidth: true }
            Button { text: compactWindow.visible ? "Hide compact view" : "Compact view"; onClicked: compactWindow.visible = !compactWindow.visible }
        }
    }

    ColumnLayout {
        anchors.fill: parent
        spacing: 0
        TabBar {
            id: mainTabs
            objectName: "mainTabs"
            Layout.fillWidth: true
            currentIndex: 0
            TabButton { objectName: "homeTab"; text: "Home" }
            TabButton {
                objectName: "machineTab"
                text: "Machine details"
                contentItem: RowLayout {
                    Label { text: "Machine details"; Layout.alignment: Qt.AlignHCenter }
                    Rectangle {
                        objectName: "machineBadge"
                        visible: wb.home.machineBadge > 0
                        implicitWidth: Math.max(20, badgeText.implicitWidth + 10)
                        implicitHeight: 20
                        radius: 10
                        color: "#c62828"
                        Label { id: badgeText; anchors.centerIn: parent; text: wb.home.machineBadge; color: "white"; font.bold: true }
                    }
                }
            }
        }
        StackLayout {
            currentIndex: mainTabs.currentIndex
            Layout.fillWidth: true
            Layout.fillHeight: true
            HomeView { id: homeView }
            MachineView {}
        }
    }

    Connections {
        target: wb
        function onFocusCard(id) {
            mainTabs.currentIndex = 0
            homeView.positionCard(id)
        }
    }

    footer: Label {
        padding: 6
        text: wb.message || "Owner notes reach Claude at its next tool step."
        elide: Text.ElideRight
        opacity: wb.message ? 1 : 0.6
    }
}

import QtQuick
import QtQuick.Layouts
import QtQuick.Controls.Basic as Controls

Rectangle {
    id: popup
    property string documentHtml: ""
    property string errorMessage: ""
    property bool busy: false
    signal linkActivated(string link)
    signal editRequested()
    signal reloadRequested()
    signal closeRequested()

    implicitWidth: 400
    implicitHeight: 480
    Layout.minimumWidth: 280
    Layout.minimumHeight: 240
    Layout.preferredWidth: 400
    Layout.preferredHeight: 480
    color: "#191919"
    radius: 8
    focus: true
    Keys.onEscapePressed: closeRequested()

    component ActionButton: Controls.Button {
        id: action
        property string actionName
        display: Controls.AbstractButton.IconOnly
        icon.width: 16
        icon.height: 16
        icon.color: enabled ? "#bcbcbc" : "#626262"
        implicitWidth: 30
        implicitHeight: 30
        Accessible.name: actionName
        Controls.ToolTip.visible: hovered
        Controls.ToolTip.text: actionName
        Controls.ToolTip.delay: 500
        contentItem: Item {
            implicitWidth: 16
            implicitHeight: 16
            Image {
                anchors.centerIn: parent
                width: 16
                height: 16
                source: action.icon.source
                opacity: action.enabled ? 1 : 0.45
            }
        }
        background: Rectangle {
            radius: 5
            color: parent.down ? "#383838" : parent.hovered ? "#2c2c2c" : "transparent"
            border.color: parent.visualFocus ? "#888888" : "transparent"
        }
    }

    ColumnLayout {
        anchors.fill: parent
        anchors.margins: 16
        spacing: 10

        RowLayout {
            Layout.fillWidth: true
            Text {
                text: "Todo"
                color: "#dedede"
                font.pixelSize: 16
                font.weight: Font.DemiBold
                Layout.fillWidth: true
            }
            ActionButton {
                actionName: "Reload"
                icon.source: "../icons/reload.svg"
                enabled: !popup.busy
                onClicked: popup.reloadRequested()
            }
            ActionButton {
                actionName: "Edit"
                icon.source: "../icons/edit.svg"
                enabled: !popup.busy
                onClicked: popup.editRequested()
            }
        }

        Rectangle { Layout.fillWidth: true; height: 1; color: "#303030" }

        Flickable {
            id: scroll
            Layout.fillWidth: true
            Layout.fillHeight: true
            clip: true
            contentWidth: Math.max(width, document.contentWidth + 12)
            contentHeight: document.height
            boundsBehavior: Flickable.StopAtBounds
            Controls.ScrollBar.vertical: Controls.ScrollBar { }
            Controls.ScrollBar.horizontal: Controls.ScrollBar { }

            TextEdit {
                id: document
                width: scroll.width - 12
                height: Math.max(implicitHeight, scroll.height)
                topPadding: 4
                bottomPadding: 8
                readOnly: true
                selectByMouse: true
                textFormat: TextEdit.RichText
                wrapMode: TextEdit.Wrap
                color: "#d2d2d2"
                selectedTextColor: "#ffffff"
                selectionColor: "#484848"
                font.pixelSize: 14
                renderType: Text.NativeRendering
                text: popup.documentHtml || (popup.busy ? "Loading..." : "No tasks yet. Use Edit to write your Markdown.")
                onLinkActivated: link => popup.linkActivated(link)
            }
        }

        Text {
            Layout.fillWidth: true
            visible: popup.errorMessage.length > 0
            text: popup.errorMessage
            textFormat: Text.PlainText
            wrapMode: Text.Wrap
            color: "#f0a8a8"
            font.pointSize: 10
            Accessible.role: Accessible.AlertMessage
        }

    }
}

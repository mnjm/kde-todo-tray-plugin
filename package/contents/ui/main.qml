import QtQuick
import QtQuick.Controls as Controls
import org.kde.plasma.core as PlasmaCore
import org.kde.plasma.plasmoid
import org.kde.plasma.plasma5support as Plasma5Support

PlasmoidItem {
    id: root

    property string documentHtml: ""
    property string revision: ""
    property string filePath: ""
    property string errorMessage: ""
    property bool busy: false

    Plasmoid.status: PlasmaCore.Types.ActiveStatus
    hideOnWindowDeactivate: true
    preferredRepresentation: compactRepresentation
    toolTipMainText: "Markdown Todo"
    toolTipSubText: filePath || "Your Markdown tasks"

    function quote(value) {
        return "'" + value.replace(/'/g, "'\\''") + "'"
    }

    function request(action, args, clearError) {
        if (busy)
            return
        busy = true
        if (clearError)
            errorMessage = ""
        const binary = decodeURIComponent(Qt.resolvedUrl("../bin/markdown-todo").toString().slice(7))
        const command = [binary, action].concat(args).map(quote).join(" ")
        runner.connectSource(command)
    }

    function activateLink(link) {
        const task = /^todo:([a-f0-9]{64}):(\d+)$/.exec(link)
        if (task) {
            request("toggle", [task[1], task[2]], true)
        } else if (/^(https?:\/\/|mailto:)/i.test(link)) {
            Qt.openUrlExternally(link)
        }
    }

    onExpandedChanged: {
        if (expanded)
            request("load", [], true)
    }

    Plasma5Support.DataSource {
        id: runner
        engine: "executable"
        connectedSources: []
        onNewData: function(sourceName, data) {
            disconnectSource(sourceName)
            root.busy = false
            try {
                if (data["exit code"] !== 0)
                    throw new Error("Backend failed. Build the widget again and check its executable.")
                const response = JSON.parse(data.stdout)
                if (response.html !== undefined)
                    root.documentHtml = response.html
                if (response.revision !== undefined)
                    root.revision = response.revision
                if (response.path !== undefined)
                    root.filePath = response.path
                if (response.error)
                    root.errorMessage = response.error
            } catch (error) {
                root.errorMessage = error.message
            }
        }
    }

    Timer {
        interval: 2000
        repeat: true
        running: root.expanded
        onTriggered: root.request("load", [root.revision], false)
    }

    compactRepresentation: Controls.ToolButton {
        icon.name: "view-task-symbolic"
        Accessible.name: "Open Markdown Todo"
        onClicked: root.expanded = !root.expanded
    }

    fullRepresentation: TodoPopup {
        documentHtml: root.documentHtml
        errorMessage: root.errorMessage
        busy: root.busy
        onLinkActivated: link => root.activateLink(link)
        onEditRequested: root.request("edit", [], true)
        onReloadRequested: root.request("load", [], true)
        onCloseRequested: root.expanded = false
    }
}

import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

Item {
    id: root

    QtObject {
        id: theme
        readonly property color page: "#f4f6f8"
        readonly property color surface: "#ffffff"
        readonly property color surfaceSoft: "#f8fafb"
        readonly property color border: "#d9e0e8"
        readonly property color borderStrong: "#b9c5d1"
        readonly property color text: "#17202c"
        readonly property color muted: "#657386"
        readonly property color faint: "#8a95a4"
        readonly property color green: "#0f6b4f"
        readonly property color greenSoft: "#e7f5ef"
        readonly property color amber: "#8a5a00"
        readonly property color amberSoft: "#fff5dd"
        readonly property color terminal: "#111827"
        readonly property color terminalText: "#dbe4ef"
    }

    QtObject {
        id: d
        readonly property string mod: "balance_attestation"
        readonly property var backend: typeof logos !== "undefined" && logos ? logos.module(mod) : null
    }

    property var proofRun: parseJson(d.backend ? d.backend.proofRunJson : "")
    property var verifyResult: parseJson(d.backend ? d.backend.verifyJson : "")
    property var gateRun: parseJson(d.backend ? d.backend.gateRunJson : "")

    function parseJson(value) {
        if (!value || value.length === 0) {
            return {}
        }
        try {
            return JSON.parse(value)
        } catch (error) {
            return {}
        }
    }

    function shortValue(value) {
        if (!value) {
            return "-"
        }
        if (value.length <= 28) {
            return value
        }
        return value.slice(0, 16) + "..." + value.slice(value.length - 9)
    }

    function statusColor(value) {
        if (value === "ok" || value === "not-applied") {
            return theme.green
        }
        if (!value || value === "idle") {
            return theme.muted
        }
        return theme.amber
    }

    function statusBackground(value) {
        if (value === "ok" || value === "not-applied") {
            return theme.greenSoft
        }
        if (!value || value === "idle") {
            return theme.surfaceSoft
        }
        return theme.amberSoft
    }

    function fieldValue(value, fallback) {
        if (value === undefined || value === null || value === "") {
            return fallback
        }
        return String(value)
    }

    Rectangle {
        anchors.fill: parent
        color: theme.page
    }

    Flickable {
        anchors.fill: parent
        clip: true
        contentWidth: width
        contentHeight: content.implicitHeight + 32

        ColumnLayout {
            id: content
            width: Math.min(parent.width - 32, 1180)
            anchors.top: parent.top
            anchors.horizontalCenter: parent.horizontalCenter
            anchors.topMargin: 18
            spacing: 12

            Rectangle {
                Layout.fillWidth: true
                radius: 8
                color: theme.surface
                border.color: theme.border
                implicitHeight: headerLayout.implicitHeight + 28

                RowLayout {
                    id: headerLayout
                    anchors.fill: parent
                    anchors.margins: 14
                    spacing: 14

                    Rectangle {
                        Layout.preferredWidth: 4
                        Layout.fillHeight: true
                        radius: 2
                        color: theme.green
                    }

                    ColumnLayout {
                        Layout.fillWidth: true
                        spacing: 2

                        Label {
                            text: "Private Balance Attestation"
                            color: theme.text
                            font.pixelSize: 24
                            font.weight: Font.DemiBold
                        }

                        Label {
                            text: "LP-0005 local flow"
                            color: theme.muted
                            font.pixelSize: 13
                        }
                    }

                    StatusPill {
                        value: d.backend && d.backend.realProving ? "RISC0_DEV_MODE=0" : "RISC0_DEV_MODE=1"
                        ok: d.backend && d.backend.realProving
                    }
                }
            }

            GridLayout {
                Layout.fillWidth: true
                columns: content.width > 920 ? 2 : 1
                columnSpacing: 12
                rowSpacing: 12

                SectionPanel {
                    title: "Environment"
                    Layout.fillWidth: true
                    Layout.minimumWidth: 360

                    FormGrid {
                        FieldLabel { text: "Repository" }
                        FormTextField {
                            text: d.backend ? d.backend.repoDir : ""
                            enabled: d.backend && !d.backend.busy
                            onEditingFinished: if (d.backend) d.backend.configureRepoDir(text)
                        }

                        FieldLabel { text: "LEZ checkout" }
                        FormTextField {
                            text: d.backend ? d.backend.lezRepoDir : ""
                            enabled: d.backend && !d.backend.busy
                            onEditingFinished: if (d.backend) d.backend.configureLezRepoDir(text)
                        }

                        FieldLabel { text: "Wallet home" }
                        FormTextField {
                            text: d.backend ? d.backend.walletHomeDir : ""
                            enabled: d.backend && !d.backend.busy
                            onEditingFinished: if (d.backend) d.backend.configureWalletHomeDir(text)
                        }

                        FieldLabel { text: "Private account" }
                        FormTextField {
                            placeholderText: "Private/<account-id>"
                            text: d.backend ? d.backend.privateAccount : ""
                            enabled: d.backend && !d.backend.busy
                            onEditingFinished: if (d.backend) d.backend.configurePrivateAccount(text)
                        }
                    }
                }

                SectionPanel {
                    title: "Gate Context"
                    Layout.fillWidth: true
                    Layout.minimumWidth: 360

                    FormGrid {
                        FieldLabel { text: "Threshold" }
                        FormTextField {
                            text: d.backend ? d.backend.threshold : "1"
                            inputMethodHints: Qt.ImhDigitsOnly
                            enabled: d.backend && !d.backend.busy
                            onEditingFinished: if (d.backend) d.backend.configureThreshold(text)
                        }

                        FieldLabel { text: "Chain id" }
                        FormTextField {
                            text: d.backend ? d.backend.chainIdHex : ""
                            enabled: d.backend && !d.backend.busy
                            onEditingFinished: if (d.backend) d.backend.configureChainIdHex(text)
                        }

                        FieldLabel { text: "Verifier id" }
                        FormTextField {
                            text: d.backend ? d.backend.verifierIdHex : ""
                            enabled: d.backend && !d.backend.busy
                            onEditingFinished: if (d.backend) d.backend.configureVerifierIdHex(text)
                        }

                        FieldLabel { text: "Gate id" }
                        FormTextField {
                            text: d.backend ? d.backend.gateIdHex : ""
                            enabled: d.backend && !d.backend.busy
                            onEditingFinished: if (d.backend) d.backend.configureGateIdHex(text)
                        }

                        FieldLabel { text: "Challenge" }
                        FormTextField {
                            text: d.backend ? d.backend.presentationChallengeHex : ""
                            enabled: d.backend && !d.backend.busy
                            onEditingFinished: if (d.backend) d.backend.configurePresentationChallengeHex(text)
                        }
                    }
                }
            }

            Rectangle {
                Layout.fillWidth: true
                radius: 8
                color: theme.surface
                border.color: theme.border
                implicitHeight: toolbar.implicitHeight + 18

                RowLayout {
                    id: toolbar
                    anchors.fill: parent
                    anchors.margins: 9
                    spacing: 8

                    ActionButton {
                        text: "Preflight"
                        enabled: d.backend && !d.backend.busy
                        onClicked: d.backend.runPreflight()
                    }

                    ActionButton {
                        text: "Generate proof"
                        primary: true
                        enabled: d.backend && !d.backend.busy
                        onClicked: d.backend.generateProof()
                    }

                    ActionButton {
                        text: "Verify envelope"
                        enabled: d.backend && !d.backend.busy && d.backend.proofRunDir.length > 0
                        onClicked: d.backend.verifyEnvelope()
                    }

                    ActionButton {
                        text: "Gate admit"
                        enabled: d.backend && !d.backend.busy && d.backend.proofRunDir.length > 0
                        onClicked: d.backend.executeGateAdmit()
                    }

                    ActionButton {
                        text: "Clear"
                        enabled: d.backend && !d.backend.busy
                        onClicked: d.backend.clearOutputs()
                    }

                    Item { Layout.fillWidth: true }

                    RunningIndicator {
                        running: d.backend ? d.backend.busy : false
                        Layout.preferredWidth: 28
                        Layout.preferredHeight: 28
                    }

                    Switch {
                        text: "Real proving"
                        checked: d.backend ? d.backend.realProving : true
                        enabled: d.backend && !d.backend.busy
                        onToggled: if (d.backend) d.backend.configureRealProving(checked)
                    }
                }
            }

            GridLayout {
                Layout.fillWidth: true
                columns: width > 780 ? 3 : 1
                columnSpacing: 12
                rowSpacing: 12

                SummaryCard {
                    title: "Proof"
                    status: root.fieldValue(root.proofRun.status, "idle")
                    main: d.backend ? root.shortValue(d.backend.proofRunDir) : "-"
                    detail: "prove " + root.fieldValue(root.proofRun.durations ? root.proofRun.durations.prove : "", "-")
                }

                SummaryCard {
                    title: "Verify"
                    status: root.fieldValue(root.verifyResult.status, "idle")
                    main: root.shortValue(root.verifyResult.context_nullifier)
                    detail: "threshold " + root.fieldValue(root.verifyResult.threshold, "-")
                }

                SummaryCard {
                    title: "Gate"
                    status: root.fieldValue(root.gateRun.status, "idle")
                    main: root.shortValue(root.gateRun.accounts ? root.gateRun.accounts.gate : root.gateRun.gate_account)
                    detail: "duplicate " + root.fieldValue(root.gateRun.duplicate_status, "-")
                }
            }

            Rectangle {
                Layout.fillWidth: true
                Layout.preferredHeight: 360
                radius: 8
                color: theme.surface
                border.color: theme.border

                ColumnLayout {
                    anchors.fill: parent
                    anchors.margins: 10
                    spacing: 8

                    TabBar {
                        id: outputTabs
                        Layout.fillWidth: true
                        background: Rectangle {
                            radius: 6
                            color: theme.surfaceSoft
                            border.color: theme.border
                        }

                        OutputTabButton { text: "Status" }
                        OutputTabButton { text: "Proof JSON" }
                        OutputTabButton { text: "Verify JSON" }
                        OutputTabButton { text: "Gate JSON" }
                    }

                    StackLayout {
                        Layout.fillWidth: true
                        Layout.fillHeight: true
                        currentIndex: outputTabs.currentIndex

                        OutputPane {
                            autoScroll: true
                            textContent: d.backend ? d.backend.status : "Backend unavailable"
                        }

                        OutputPane {
                            textContent: d.backend ? d.backend.proofRunJson : ""
                        }

                        OutputPane {
                            textContent: d.backend ? d.backend.verifyJson : ""
                        }

                        OutputPane {
                            textContent: d.backend ? d.backend.gateRunJson : ""
                        }
                    }
                }
            }
        }
    }

    component FieldLabel: Label {
        color: theme.muted
        font.pixelSize: 12
        font.weight: Font.Medium
        Layout.alignment: Qt.AlignVCenter
    }

    component FormTextField: TextField {
        Layout.fillWidth: true
        selectByMouse: true
        color: theme.text
        selectionColor: "#c8e8dc"
        selectedTextColor: theme.text
        font.pixelSize: 13
        leftPadding: 10
        rightPadding: 10
        background: Rectangle {
            radius: 6
            color: parent.enabled ? theme.surface : "#eef1f4"
            border.color: parent.activeFocus ? theme.green : theme.borderStrong
        }
    }

    component FormGrid: GridLayout {
        Layout.fillWidth: true
        columns: 2
        columnSpacing: 12
        rowSpacing: 8
    }

    component SectionPanel: Rectangle {
        id: panel
        property string title: ""
        default property alias content: body.data
        radius: 8
        color: theme.surface
        border.color: theme.border
        implicitHeight: body.implicitHeight + 48

        ColumnLayout {
            id: body
            anchors.fill: parent
            anchors.margins: 12
            spacing: 10

            Label {
                text: panel.title
                color: theme.text
                font.pixelSize: 13
                font.weight: Font.DemiBold
            }
        }
    }

    component ActionButton: Button {
        id: control
        property bool primary: false
        implicitHeight: 40
        implicitWidth: Math.max(112, label.implicitWidth + 28)

        contentItem: Text {
            id: label
            text: control.text
            color: !control.enabled ? theme.faint : (control.primary ? "#ffffff" : theme.text)
            horizontalAlignment: Text.AlignHCenter
            verticalAlignment: Text.AlignVCenter
            font.pixelSize: 13
            font.weight: Font.Medium
        }

        background: Rectangle {
            radius: 6
            color: !control.enabled ? "#eef1f4" : (control.primary ? theme.green : theme.surfaceSoft)
            border.color: !control.enabled ? theme.border : (control.primary ? theme.green : theme.borderStrong)
        }
    }

    component StatusPill: Rectangle {
        property string value: ""
        property bool ok: false
        radius: 6
        color: ok ? theme.greenSoft : theme.amberSoft
        border.color: ok ? "#abd8c8" : "#efd18b"
        implicitWidth: pillLabel.implicitWidth + 18
        implicitHeight: pillLabel.implicitHeight + 8

        Label {
            id: pillLabel
            anchors.centerIn: parent
            text: parent.value
            color: parent.ok ? theme.green : theme.amber
            font.pixelSize: 12
            font.weight: Font.Medium
        }
    }

    component RunningIndicator: Item {
        id: indicator
        property bool running: false
        visible: running

        Rectangle {
            id: ring
            anchors.centerIn: parent
            width: 24
            height: 24
            radius: 12
            color: "transparent"
            border.color: theme.green
            border.width: 2
            opacity: indicator.running ? 1 : 0

            Rectangle {
                width: 7
                height: 7
                radius: 3.5
                color: theme.green
                anchors.horizontalCenter: parent.horizontalCenter
                anchors.top: parent.top
                anchors.topMargin: -2
            }

            NumberAnimation on rotation {
                from: 0
                to: 360
                duration: 850
                loops: Animation.Infinite
                running: indicator.running
            }
        }
    }

    component SummaryCard: Rectangle {
        property string title: ""
        property string status: "idle"
        property string main: "-"
        property string detail: "-"
        Layout.fillWidth: true
        Layout.preferredHeight: 138
        radius: 8
        color: theme.surface
        border.color: theme.border

        ColumnLayout {
            anchors.fill: parent
            anchors.margins: 12
            spacing: 8

            RowLayout {
                Layout.fillWidth: true
                Label {
                    Layout.fillWidth: true
                    text: title
                    color: theme.text
                    font.pixelSize: 13
                    font.weight: Font.DemiBold
                }
                Rectangle {
                    radius: 5
                    color: root.statusBackground(status)
                    implicitWidth: statusLabel.implicitWidth + 14
                    implicitHeight: statusLabel.implicitHeight + 6
                    Label {
                        id: statusLabel
                        anchors.centerIn: parent
                        text: status
                        color: root.statusColor(status)
                        font.pixelSize: 12
                        font.weight: Font.Medium
                    }
                }
            }

            Label {
                Layout.fillWidth: true
                text: main
                color: theme.text
                elide: Text.ElideMiddle
                font.pixelSize: 13
            }

            Label {
                Layout.fillWidth: true
                text: detail
                color: theme.muted
                elide: Text.ElideRight
                font.pixelSize: 12
            }
        }
    }

    component OutputTabButton: TabButton {
        id: tab
        implicitHeight: 38
        contentItem: Text {
            text: tab.text
            color: tab.checked ? "#ffffff" : theme.muted
            horizontalAlignment: Text.AlignHCenter
            verticalAlignment: Text.AlignVCenter
            font.pixelSize: 13
            font.weight: tab.checked ? Font.DemiBold : Font.Medium
        }
        background: Rectangle {
            radius: 6
            color: tab.checked ? theme.terminal : "transparent"
            border.color: "transparent"
        }
    }

    component OutputPane: ScrollView {
        id: pane
        property string textContent: ""
        property bool autoScroll: false
        clip: true
        ScrollBar.vertical.policy: ScrollBar.AsNeeded
        ScrollBar.horizontal.policy: ScrollBar.AlwaysOff

        TextArea {
            id: outputText
            text: pane.textContent
            readOnly: true
            selectByMouse: true
            wrapMode: TextEdit.Wrap
            textFormat: TextEdit.PlainText
            color: theme.terminalText
            padding: 12
            font.family: "Menlo"
            font.pixelSize: 12
            background: Rectangle {
                radius: 6
                color: theme.terminal
                border.color: "#253044"
            }

            onTextChanged: {
                if (pane.autoScroll) {
                    cursorPosition = length
                }
            }
        }
    }
}

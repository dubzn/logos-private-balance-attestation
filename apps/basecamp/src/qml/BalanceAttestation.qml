import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

Item {
    id: root

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
        if (value.length <= 24) {
            return value
        }
        return value.slice(0, 14) + "..." + value.slice(value.length - 8)
    }

    function statusColor(value) {
        if (value === "ok" || value === "not-applied") {
            return "#0f6b4f"
        }
        if (!value) {
            return "#6b7280"
        }
        return "#7a4b00"
    }

    function fieldValue(value, fallback) {
        if (value === undefined || value === null || value === "") {
            return fallback
        }
        return String(value)
    }

    Flickable {
        anchors.fill: parent
        clip: true
        contentWidth: width
        contentHeight: content.implicitHeight + 32

        ColumnLayout {
            id: content
            width: Math.min(parent.width - 32, 1120)
            anchors.top: parent.top
            anchors.horizontalCenter: parent.horizontalCenter
            anchors.topMargin: 16
            spacing: 12

            RowLayout {
                Layout.fillWidth: true
                spacing: 10

                ColumnLayout {
                    Layout.fillWidth: true
                    spacing: 2

                    Label {
                        text: "Private Balance Attestation"
                        font.pixelSize: 24
                        font.weight: Font.DemiBold
                    }

                    Label {
                        text: "LP-0005 local flow"
                        color: "#5f6875"
                    }
                }

                Rectangle {
                    radius: 6
                    color: "#e9f6f1"
                    border.color: "#b7ddcf"
                    implicitWidth: modeLabel.implicitWidth + 18
                    implicitHeight: modeLabel.implicitHeight + 8

                    Label {
                        id: modeLabel
                        anchors.centerIn: parent
                        text: d.backend && d.backend.realProving ? "RISC0_DEV_MODE=0" : "RISC0_DEV_MODE=1"
                        color: "#0f6b4f"
                        font.pixelSize: 12
                        font.weight: Font.Medium
                    }
                }
            }

            Rectangle {
                Layout.fillWidth: true
                radius: 8
                color: "#ffffff"
                border.color: "#d8dee8"
                implicitHeight: environmentLayout.implicitHeight + 24

                GridLayout {
                    id: environmentLayout
                    anchors.fill: parent
                    anchors.margins: 12
                    columns: 2
                    columnSpacing: 12
                    rowSpacing: 8

                    Label { text: "Repository"; color: "#5f6875" }
                    TextField {
                        Layout.fillWidth: true
                        text: d.backend ? d.backend.repoDir : ""
                        enabled: d.backend && !d.backend.busy
                        onEditingFinished: if (d.backend) d.backend.configureRepoDir(text)
                    }

                    Label { text: "LEZ checkout"; color: "#5f6875" }
                    TextField {
                        Layout.fillWidth: true
                        text: d.backend ? d.backend.lezRepoDir : ""
                        enabled: d.backend && !d.backend.busy
                        onEditingFinished: if (d.backend) d.backend.configureLezRepoDir(text)
                    }

                    Label { text: "Wallet home"; color: "#5f6875" }
                    TextField {
                        Layout.fillWidth: true
                        text: d.backend ? d.backend.walletHomeDir : ""
                        enabled: d.backend && !d.backend.busy
                        onEditingFinished: if (d.backend) d.backend.configureWalletHomeDir(text)
                    }

                    Label { text: "Private account"; color: "#5f6875" }
                    TextField {
                        Layout.fillWidth: true
                        placeholderText: "Private/<account-id>"
                        text: d.backend ? d.backend.privateAccount : ""
                        enabled: d.backend && !d.backend.busy
                        onEditingFinished: if (d.backend) d.backend.configurePrivateAccount(text)
                    }
                }
            }

            Rectangle {
                Layout.fillWidth: true
                radius: 8
                color: "#ffffff"
                border.color: "#d8dee8"
                implicitHeight: contextLayout.implicitHeight + 24

                GridLayout {
                    id: contextLayout
                    anchors.fill: parent
                    anchors.margins: 12
                    columns: 2
                    columnSpacing: 12
                    rowSpacing: 8

                    Label { text: "Threshold"; color: "#5f6875" }
                    TextField {
                        Layout.fillWidth: true
                        text: d.backend ? d.backend.threshold : "1"
                        inputMethodHints: Qt.ImhDigitsOnly
                        enabled: d.backend && !d.backend.busy
                        onEditingFinished: if (d.backend) d.backend.configureThreshold(text)
                    }

                    Label { text: "Chain id"; color: "#5f6875" }
                    TextField {
                        Layout.fillWidth: true
                        text: d.backend ? d.backend.chainIdHex : ""
                        enabled: d.backend && !d.backend.busy
                        onEditingFinished: if (d.backend) d.backend.configureChainIdHex(text)
                    }

                    Label { text: "Verifier id"; color: "#5f6875" }
                    TextField {
                        Layout.fillWidth: true
                        text: d.backend ? d.backend.verifierIdHex : ""
                        enabled: d.backend && !d.backend.busy
                        onEditingFinished: if (d.backend) d.backend.configureVerifierIdHex(text)
                    }

                    Label { text: "Gate id"; color: "#5f6875" }
                    TextField {
                        Layout.fillWidth: true
                        text: d.backend ? d.backend.gateIdHex : ""
                        enabled: d.backend && !d.backend.busy
                        onEditingFinished: if (d.backend) d.backend.configureGateIdHex(text)
                    }

                    Label { text: "Challenge"; color: "#5f6875" }
                    TextField {
                        Layout.fillWidth: true
                        text: d.backend ? d.backend.presentationChallengeHex : ""
                        enabled: d.backend && !d.backend.busy
                        onEditingFinished: if (d.backend) d.backend.configurePresentationChallengeHex(text)
                    }
                }
            }

            RowLayout {
                Layout.fillWidth: true
                spacing: 8

                Button {
                    text: "Preflight"
                    enabled: d.backend && !d.backend.busy
                    onClicked: d.backend.runPreflight()
                }

                Button {
                    text: "Generate proof"
                    enabled: d.backend && !d.backend.busy
                    onClicked: d.backend.generateProof()
                }

                Button {
                    text: "Verify envelope"
                    enabled: d.backend && !d.backend.busy && d.backend.proofRunDir.length > 0
                    onClicked: d.backend.verifyEnvelope()
                }

                Button {
                    text: "Gate admit"
                    enabled: d.backend && !d.backend.busy && d.backend.proofRunDir.length > 0
                    onClicked: d.backend.executeGateAdmit()
                }

                Button {
                    text: "Clear"
                    enabled: d.backend && !d.backend.busy
                    onClicked: d.backend.clearOutputs()
                }

                Item { Layout.fillWidth: true }

                Switch {
                    text: "Real proving"
                    checked: d.backend ? d.backend.realProving : true
                    enabled: d.backend && !d.backend.busy
                    onToggled: if (d.backend) d.backend.configureRealProving(checked)
                }

                BusyIndicator {
                    running: d.backend ? d.backend.busy : false
                    visible: running
                }
            }

            GridLayout {
                Layout.fillWidth: true
                columns: width > 760 ? 3 : 1
                columnSpacing: 12
                rowSpacing: 12

                Rectangle {
                    Layout.fillWidth: true
                    Layout.preferredHeight: 168
                    radius: 8
                    color: "#ffffff"
                    border.color: "#d8dee8"

                    ColumnLayout {
                        anchors.fill: parent
                        anchors.margins: 12
                        spacing: 8

                        Label { text: "Proof"; font.weight: Font.DemiBold }
                        Label {
                            Layout.fillWidth: true
                            text: root.fieldValue(root.proofRun.status, "idle")
                            color: root.statusColor(root.proofRun.status)
                            font.weight: Font.Medium
                        }
                        Label {
                            Layout.fillWidth: true
                            text: d.backend ? root.shortValue(d.backend.proofRunDir) : "-"
                            elide: Text.ElideMiddle
                        }
                        Label {
                            Layout.fillWidth: true
                            text: "prove " + root.fieldValue(root.proofRun.durations ? root.proofRun.durations.prove : "", "-")
                            color: "#5f6875"
                        }
                    }
                }

                Rectangle {
                    Layout.fillWidth: true
                    Layout.preferredHeight: 168
                    radius: 8
                    color: "#ffffff"
                    border.color: "#d8dee8"

                    ColumnLayout {
                        anchors.fill: parent
                        anchors.margins: 12
                        spacing: 8

                        Label { text: "Verify"; font.weight: Font.DemiBold }
                        Label {
                            Layout.fillWidth: true
                            text: root.fieldValue(root.verifyResult.status, "idle")
                            color: root.statusColor(root.verifyResult.status)
                            font.weight: Font.Medium
                        }
                        Label {
                            Layout.fillWidth: true
                            text: root.shortValue(root.verifyResult.context_nullifier)
                            elide: Text.ElideMiddle
                        }
                        Label {
                            Layout.fillWidth: true
                            text: "threshold " + root.fieldValue(root.verifyResult.threshold, "-")
                            color: "#5f6875"
                        }
                    }
                }

                Rectangle {
                    Layout.fillWidth: true
                    Layout.preferredHeight: 168
                    radius: 8
                    color: "#ffffff"
                    border.color: "#d8dee8"

                    ColumnLayout {
                        anchors.fill: parent
                        anchors.margins: 12
                        spacing: 8

                        Label { text: "Gate"; font.weight: Font.DemiBold }
                        Label {
                            Layout.fillWidth: true
                            text: root.fieldValue(root.gateRun.status, "idle")
                            color: root.statusColor(root.gateRun.status)
                            font.weight: Font.Medium
                        }
                        Label {
                            Layout.fillWidth: true
                            text: root.shortValue(root.gateRun.accounts ? root.gateRun.accounts.gate : root.gateRun.gate_account)
                            elide: Text.ElideMiddle
                        }
                        Label {
                            Layout.fillWidth: true
                            text: "duplicate " + root.fieldValue(root.gateRun.duplicate_status, "-")
                            color: "#5f6875"
                        }
                    }
                }
            }

            Rectangle {
                Layout.fillWidth: true
                radius: 8
                color: "#ffffff"
                border.color: "#d8dee8"
                implicitHeight: outputLayout.implicitHeight + 24

                ColumnLayout {
                    id: outputLayout
                    anchors.fill: parent
                    anchors.margins: 12
                    spacing: 8

                    TabBar {
                        id: outputTabs
                        Layout.fillWidth: true
                        TabButton { text: "Status" }
                        TabButton { text: "Proof JSON" }
                        TabButton { text: "Verify JSON" }
                        TabButton { text: "Gate JSON" }
                    }

                    StackLayout {
                        Layout.fillWidth: true
                        Layout.preferredHeight: 260
                        currentIndex: outputTabs.currentIndex

                        TextArea {
                            readOnly: true
                            wrapMode: TextEdit.Wrap
                            text: d.backend ? d.backend.status : "Backend unavailable"
                        }

                        TextArea {
                            readOnly: true
                            wrapMode: TextEdit.Wrap
                            text: d.backend ? d.backend.proofRunJson : ""
                        }

                        TextArea {
                            readOnly: true
                            wrapMode: TextEdit.Wrap
                            text: d.backend ? d.backend.verifyJson : ""
                        }

                        TextArea {
                            readOnly: true
                            wrapMode: TextEdit.Wrap
                            text: d.backend ? d.backend.gateRunJson : ""
                        }
                    }
                }
            }
        }
    }
}

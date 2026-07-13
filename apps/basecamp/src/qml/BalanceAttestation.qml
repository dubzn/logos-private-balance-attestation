import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

Item {
    id: root

    QtObject {
        id: theme
        readonly property color page: "#e8edf5"
        readonly property color surface: "#ffffff"
        readonly property color surfaceSoft: "#f7fafc"
        readonly property color border: "#d8e1ea"
        readonly property color borderStrong: "#b8c6d3"
        readonly property color text: "#101827"
        readonly property color muted: "#64748b"
        readonly property color faint: "#9aa8b7"
        readonly property color green: "#0f8f74"
        readonly property color greenSoft: "#e5f6f1"
        readonly property color blue: "#1476bd"
        readonly property color blueSoft: "#e6f2fb"
        readonly property color amber: "#9a6500"
        readonly property color amberSoft: "#fff5dd"
        readonly property color pending: "#a7d1f5"
        readonly property color pendingSoft: "#f1f7fd"
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
    property var deliveryVerifyResult: parseJson(d.backend ? d.backend.deliveryVerifyJson : "")
    property bool showConsole: false
    property bool preflightPassed: false
    property int selectedStep: 0
    property int runningStep: -1
    property int failedStep: -1
    property bool deliveryBusy: false
    property string deliveryAction: ""
    readonly property var steps: [
        {"label": "STEP 1", "title": "Setup", "detail": "Paths and gate"},
        {"label": "STEP 2", "title": "Account", "detail": "Wallet preflight"},
        {"label": "STEP 3", "title": "Proof", "detail": "Generate envelope"},
        {"label": "STEP 4", "title": "Verify", "detail": "Local verifier"},
        {"label": "STEP 5", "title": "Gate", "detail": "LEZ admit"},
        {"label": "STEP 6", "title": "Delivery", "detail": "Message proof"}
    ]

    readonly property bool setupReady: d.backend
        && d.backend.repoDir.length > 0
        && d.backend.lezRepoDir.length > 0
        && d.backend.walletHomeDir.length > 0
        && d.backend.privateAccount.length > 0
        && d.backend.threshold.length > 0
    readonly property bool accountReady: setupReady && (
        preflightPassed
        || (d.backend && d.backend.status.indexOf("Local private account is ready") >= 0)
        || (d.backend && d.backend.status.indexOf("Private balance already satisfies") >= 0)
        || (d.backend && d.backend.status.indexOf("Wallet/sequencer preflight ok") >= 0)
        || proofComplete
    )
    readonly property bool proofComplete: d.backend && (
        root.fieldValue(root.proofRun.status, "") === "ok"
        || d.backend.proofRunJson.length > 2
    )
    readonly property bool verifyComplete: root.fieldValue(root.verifyResult.status, "") === "ok"
    readonly property bool gateComplete: root.fieldValue(root.gateRun.status, "") === "ok"
        || root.fieldValue(root.gateRun.verify_status, "") === "ok"
        || root.fieldValue(root.gateRun.duplicate_status, "") === "not-applied"
    readonly property bool deliveryNodeReady: d.backend && (
        d.backend.deliveryNodeStarted
        || d.backend.deliveryPeerId.length > 0
    )
    readonly property bool deliverySubscribed: d.backend && d.backend.deliverySubscribed
    readonly property bool deliveryReceivedReady: d.backend && d.backend.deliveryReceived && d.backend.deliveryMessageJson.length > 0
    readonly property bool deliveryMessageReady: d.backend && d.backend.deliveryMessageJson.length > 0
    readonly property bool deliveryComplete: root.fieldValue(root.deliveryVerifyResult.status, "") === "ok"

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

    function fieldValue(value, fallback) {
        if (value === undefined || value === null || value === "") {
            return fallback
        }
        return String(value)
    }

    function shortValue(value) {
        if (!value) {
            return "-"
        }
        if (value.length <= 34) {
            return value
        }
        return value.slice(0, 18) + "..." + value.slice(value.length - 10)
    }

    function statusTail(value) {
        if (!value || value.length === 0) {
            return "Ready"
        }
        var cleaned = String(value).trim()
        if (cleaned.length <= 180) {
            return cleaned
        }
        return "..." + cleaned.slice(cleaned.length - 177)
    }

    function statusLooksFailed(value) {
        var text = String(value || "").toLowerCase()
        return text.indexOf("error:") >= 0
            || text.indexOf(" failed") >= 0
            || text.indexOf("failed ") >= 0
            || text.indexOf("panicked at") >= 0
            || text.indexOf("command failed") >= 0
            || text.indexOf("exit status") >= 0
    }

    function stepComplete(index) {
        if (index === 0) return setupReady
        if (index === 1) return accountReady
        if (index === 2) return proofComplete
        if (index === 3) return verifyComplete
        if (index === 4) return gateComplete
        if (index === 5) return deliveryComplete
        return false
    }

    function nextOpenStep() {
        for (var i = 0; i < steps.length; i++) {
            if (!stepComplete(i)) {
                return i
            }
        }
        return steps.length - 1
    }

    function stepState(index) {
        if (failedStep === index && !stepComplete(index)) {
            return "failed"
        }
        if (runningStep === index && ((d.backend && d.backend.busy) || deliveryBusy)) {
            return "running"
        }
        if (stepComplete(index)) {
            return "completed"
        }
        if (index === nextOpenStep()) {
            return "current"
        }
        return "pending"
    }

    function stepAccent(index) {
        var state = stepState(index)
        if (state === "completed") return theme.green
        if (state === "failed") return theme.amber
        if (state === "running" || state === "current") return theme.blue
        return theme.pending
    }

    function stepStatusText(index) {
        var state = stepState(index)
        if (state === "completed") return "Completed"
        if (state === "failed") return "Failed"
        if (state === "running") return "In progress"
        if (state === "current") return "Next"
        return "Pending"
    }

    function stepStatusColor(index) {
        var state = stepState(index)
        if (state === "completed") return theme.green
        if (state === "failed") return theme.amber
        if (state === "running" || state === "current") return theme.blue
        return theme.faint
    }

    function startStep(index) {
        selectedStep = index
        runningStep = index
        failedStep = -1
        showConsole = true
    }

    function runDeliveryAction(action, callback) {
        if (!d.backend || d.backend.busy || deliveryBusy) {
            return
        }
        selectedStep = 5
        runningStep = 5
        failedStep = -1
        showConsole = true
        deliveryBusy = true
        deliveryAction = action
        Qt.callLater(function() {
            callback()
            deliveryReleaseTimer.restart()
        })
    }

    function finishDeliveryAction(failed) {
        deliveryBusy = false
        deliveryAction = ""
        if (runningStep === 5) {
            runningStep = -1
        }
        if (failed) {
            failedStep = 5
            selectedStep = 5
        }
    }

    Timer {
        id: deliveryReleaseTimer
        interval: 120000
        repeat: false
        onTriggered: root.finishDeliveryAction(root.statusLooksFailed(d.backend ? d.backend.deliveryStatus : ""))
    }

    Connections {
        target: d.backend
        function onStatusChanged() {
            var status = d.backend ? d.backend.status : ""
            if (status.indexOf("Wallet/sequencer preflight ok") >= 0
                || status.indexOf("Local private account is ready") >= 0
                || status.indexOf("Private balance already satisfies") >= 0) {
                root.preflightPassed = true
            }
            if (root.runningStep >= 0 && root.statusLooksFailed(status)) {
                root.failedStep = root.runningStep
            }
        }

        function onDeliveryStatusChanged() {
            var status = d.backend ? d.backend.deliveryStatus : ""
            if (root.runningStep === 5 && root.statusLooksFailed(status)) {
                root.finishDeliveryAction(true)
            } else if (root.deliveryBusy
                    && status.length > 0
                    && status.indexOf("...") < 0
                    && status.indexOf("Preparing Delivery") < 0) {
                root.finishDeliveryAction(false)
            }
        }

        function onBusyChanged() {
            if (d.backend && !d.backend.busy) {
                var finishedStep = root.runningStep
                root.runningStep = -1
                if (root.failedStep >= 0) {
                    root.selectedStep = root.failedStep
                } else if (finishedStep >= 0) {
                    root.selectedStep = root.nextOpenStep()
                }
            }
        }
    }

    Rectangle {
        anchors.fill: parent
        color: theme.page
    }

    Flickable {
        anchors.fill: parent
        clip: true
        contentWidth: width
        contentHeight: content.implicitHeight + 40

        ColumnLayout {
            id: content
            width: Math.min(parent.width - 36, 1160)
            anchors.top: parent.top
            anchors.horizontalCenter: parent.horizontalCenter
            anchors.topMargin: 18
            spacing: 14

            Rectangle {
                Layout.fillWidth: true
                radius: 8
                color: theme.surface
                border.color: theme.border
                implicitHeight: headerLayout.implicitHeight + 26

                RowLayout {
                    id: headerLayout
                    anchors.fill: parent
                    anchors.margins: 13
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

                    Item {
                        Layout.fillWidth: true
                    }

                    StatusPill {
                        value: d.backend && d.backend.realProving ? "Real proving" : "Dev proving"
                        tone: d.backend && d.backend.realProving ? "green" : "amber"
                    }
                }
            }

            Rectangle {
                Layout.fillWidth: true
                radius: 8
                color: theme.surface
                border.color: theme.border
                implicitHeight: stepperFlick.implicitHeight + 28

                Flickable {
                    id: stepperFlick
                    anchors.fill: parent
                    anchors.margins: 14
                    clip: true
                    interactive: contentWidth > width
                    contentWidth: Math.max(width, stepperRow.implicitWidth)
                    contentHeight: stepperRow.implicitHeight
                    implicitHeight: 126

                    RowLayout {
                        id: stepperRow
                        width: Math.max(stepperFlick.width, implicitWidth)
                        spacing: 0

                        Repeater {
                            model: root.steps.length
                            delegate: RowLayout {
                                spacing: 6

                                StepNode {
                                    index: modelData
                                    label: root.steps[modelData].label
                                    title: root.steps[modelData].title
                                    detail: root.steps[modelData].detail
                                }

                                Rectangle {
                                    visible: modelData < root.steps.length - 1
                                    Layout.preferredWidth: 38
                                    Layout.preferredHeight: 3
                                    radius: 2
                                    color: root.stepComplete(modelData) ? theme.green : "#c8def1"
                                }
                            }
                        }
                    }
                }
            }

            GuidedStepCard {
                stepIndex: 0
                title: "Setup"
                subtitle: "Repository, wallet, account, and gate context"
                visible: root.selectedStep === 0

                ColumnLayout {
                    Layout.fillWidth: true
                    spacing: 12

                    GridLayout {
                        Layout.fillWidth: true
                        columns: content.width > 820 ? 4 : 2
                        columnSpacing: 12
                        rowSpacing: 8

                        FieldLabel { text: "Repository" }
                        FormTextField {
                            Layout.columnSpan: content.width > 820 ? 3 : 1
                            text: d.backend ? d.backend.repoDir : ""
                            enabled: d.backend && !d.backend.busy
                            onEditingFinished: if (d.backend) d.backend.configureRepoDir(text)
                        }

                        FieldLabel { text: "LEZ checkout" }
                        FormTextField {
                            Layout.columnSpan: content.width > 820 ? 3 : 1
                            text: d.backend ? d.backend.lezRepoDir : ""
                            enabled: d.backend && !d.backend.busy
                            onEditingFinished: if (d.backend) d.backend.configureLezRepoDir(text)
                        }

                        FieldLabel { text: "Wallet home" }
                        FormTextField {
                            Layout.columnSpan: content.width > 820 ? 3 : 1
                            text: d.backend ? d.backend.walletHomeDir : ""
                            enabled: d.backend && !d.backend.busy
                            onEditingFinished: if (d.backend) d.backend.configureWalletHomeDir(text)
                        }

                        FieldLabel { text: "Private account" }
                        FormTextField {
                            Layout.columnSpan: content.width > 820 ? 3 : 1
                            placeholderText: "Private/<account-id>"
                            text: d.backend ? d.backend.privateAccount : ""
                            enabled: d.backend && !d.backend.busy
                            onEditingFinished: if (d.backend) d.backend.configurePrivateAccount(text)
                        }

                        FieldLabel { text: "Threshold" }
                        FormTextField {
                            text: d.backend ? d.backend.threshold : "1"
                            inputMethodHints: Qt.ImhDigitsOnly
                            enabled: d.backend && !d.backend.busy
                            onEditingFinished: if (d.backend) d.backend.configureThreshold(text)
                        }

                        FieldLabel { text: "Real proving" }
                        Switch {
                            checked: d.backend ? d.backend.realProving : true
                            enabled: d.backend && !d.backend.busy
                            onToggled: if (d.backend) d.backend.configureRealProving(checked)
                        }
                    }

                    DetailsExpander {
                        title: "Gate context"

                        GridLayout {
                            Layout.fillWidth: true
                            columns: 2
                            columnSpacing: 12
                            rowSpacing: 8

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

                    RowLayout {
                        Layout.fillWidth: true

                        StatusLine {
                            Layout.fillWidth: true
                            text: root.setupReady
                                ? "Setup complete. Continue to Account to check wallet and sequencer."
                                : "Fill repository, LEZ checkout, wallet home, private account, and threshold."
                        }

                        ActionButton {
                            text: "Continue to Account"
                            primary: root.setupReady
                            enabled: root.setupReady && d.backend && !d.backend.busy
                            onClicked: root.selectedStep = 1
                        }
                    }
                }
            }

            GuidedStepCard {
                stepIndex: 1
                title: "Account"
                subtitle: "Check wallet, sequencer, and selected private account"
                visible: root.selectedStep === 1

                ColumnLayout {
                    Layout.fillWidth: true
                    spacing: 10

                    RowLayout {
                        Layout.fillWidth: true
                        spacing: 10

                        ActionButton {
                            text: "Run preflight"
                            primary: root.stepState(1) === "current"
                            enabled: d.backend && !d.backend.busy && root.setupReady
                            onClicked: {
                                root.startStep(1)
                                d.backend.runPreflight()
                            }
                        }

                        StatusLine {
                            Layout.fillWidth: true
                            text: root.accountReady ? "Private balance ready for threshold " + (d.backend ? d.backend.threshold : "-") : root.statusTail(d.backend ? d.backend.status : "")
                        }
                    }

                    InfoRow { name: "Wallet home"; value: d.backend ? d.backend.walletHomeDir : "-" }
                    InfoRow { name: "Private account"; value: d.backend ? d.backend.privateAccount : "-" }
                }
            }

            GuidedStepCard {
                stepIndex: 2
                title: "Proof"
                subtitle: "Generate the public proof envelope from wallet state"
                visible: root.selectedStep === 2

                ColumnLayout {
                    Layout.fillWidth: true
                    spacing: 10

                    RowLayout {
                        Layout.fillWidth: true
                        spacing: 10

                        ActionButton {
                            text: "Generate proof"
                            primary: root.stepState(2) === "current"
                            enabled: d.backend && !d.backend.busy && root.setupReady
                            onClicked: {
                                root.startStep(2)
                                d.backend.generateProof()
                            }
                        }

                        ColumnLayout {
                            Layout.fillWidth: true
                            spacing: 3

                            StatusLine {
                                Layout.fillWidth: true
                                text: root.proofComplete ? "Envelope ready: " + root.shortValue(d.backend ? d.backend.proofRunDir : "") : root.statusTail(d.backend ? d.backend.status : "")
                            }

                            Label {
                                Layout.fillWidth: true
                                text: "Prove duration: " + root.fieldValue(root.proofRun.durations ? root.proofRun.durations.prove : "", "-")
                                color: theme.muted
                                font.pixelSize: 12
                                elide: Text.ElideRight
                            }
                        }
                    }

                    InfoRow { name: "Run directory"; value: d.backend ? d.backend.proofRunDir : "-" }
                    InfoRow { name: "Envelope"; value: d.backend && d.backend.proofRunDir.length > 0 ? d.backend.proofRunDir + "/envelope.json" : "-" }
                }
            }

            GuidedStepCard {
                stepIndex: 3
                title: "Verify"
                subtitle: "Verify the envelope locally before gate admission"
                visible: root.selectedStep === 3

                ColumnLayout {
                    Layout.fillWidth: true
                    spacing: 10

                    RowLayout {
                        Layout.fillWidth: true
                        spacing: 10

                        ActionButton {
                            text: "Verify envelope"
                            primary: root.stepState(3) === "current"
                            enabled: d.backend && !d.backend.busy && root.proofComplete
                            onClicked: {
                                root.startStep(3)
                                d.backend.verifyEnvelope()
                            }
                        }

                        StatusLine {
                            Layout.fillWidth: true
                            text: root.verifyComplete ? "Verified nullifier: " + root.shortValue(root.verifyResult.context_nullifier) : root.statusTail(d.backend ? d.backend.status : "")
                        }
                    }

                    InfoRow { name: "Threshold"; value: root.fieldValue(root.verifyResult.threshold, "-") }
                    InfoRow { name: "Context nullifier"; value: root.fieldValue(root.verifyResult.context_nullifier, "-") }
                }
            }

            GuidedStepCard {
                stepIndex: 4
                title: "Gate"
                subtitle: "Submit the current Workable LEZ gate admit flow"
                visible: root.selectedStep === 4

                ColumnLayout {
                    Layout.fillWidth: true
                    spacing: 10

                    RowLayout {
                        Layout.fillWidth: true
                        spacing: 10

                        ActionButton {
                            text: "Gate admit"
                            primary: root.stepState(4) === "current"
                            enabled: d.backend && !d.backend.busy && root.verifyComplete
                            onClicked: {
                                root.startStep(4)
                                d.backend.executeGateAdmit()
                            }
                        }

                        StatusLine {
                            Layout.fillWidth: true
                            text: root.gateComplete
                                ? "Gate account: " + root.shortValue(root.gateRun.accounts ? root.gateRun.accounts.gate : root.gateRun.gate_account)
                                : root.statusTail(d.backend ? d.backend.status : "")
                        }
                    }

                    InfoRow { name: "Gate account"; value: root.fieldValue(root.gateRun.accounts ? root.gateRun.accounts.gate : root.gateRun.gate_account, "-") }
                    InfoRow { name: "Duplicate admit"; value: root.fieldValue(root.gateRun.duplicate_status, "-") }
                }
            }

            GuidedStepCard {
                stepIndex: 5
                title: "Delivery"
                subtitle: "Send or receive the same proof message over Logos Delivery"
                visible: root.selectedStep === 5

                ColumnLayout {
                    Layout.fillWidth: true
                    spacing: 12

                    GridLayout {
                        Layout.fillWidth: true
                        columns: content.width > 820 ? 4 : 2
                        columnSpacing: 12
                        rowSpacing: 8

                        FieldLabel { text: "Preset" }
                        FormTextField {
                            text: d.backend ? d.backend.deliveryPreset : "logos.test"
                            enabled: d.backend && !d.backend.busy && !root.deliveryBusy && !root.deliveryNodeReady
                            onEditingFinished: if (d.backend) d.backend.configureDeliveryPreset(text)
                        }

                        FieldLabel { text: "Mode" }
                        FormTextField {
                            text: d.backend ? d.backend.deliveryMode : "Core"
                            enabled: d.backend && !d.backend.busy && !root.deliveryBusy && !root.deliveryNodeReady
                            onEditingFinished: if (d.backend) d.backend.configureDeliveryMode(text)
                        }

                        FieldLabel { text: "Topic" }
                        FormTextField {
                            Layout.columnSpan: content.width > 820 ? 3 : 1
                            text: d.backend ? d.backend.deliveryTopic : ""
                            enabled: d.backend && !d.backend.busy && !root.deliveryBusy && !root.deliverySubscribed
                            onEditingFinished: if (d.backend) d.backend.configureDeliveryTopic(text)
                        }

                        FieldLabel { text: "Group" }
                        FormTextField {
                            text: d.backend ? d.backend.deliveryGroupId : ""
                            enabled: d.backend && !d.backend.busy && !root.deliveryBusy
                            onEditingFinished: if (d.backend) d.backend.configureDeliveryGroupId(text)
                        }

                        FieldLabel { text: "Sender" }
                        FormTextField {
                            text: d.backend ? d.backend.deliverySender : ""
                            enabled: d.backend && !d.backend.busy && !root.deliveryBusy
                            onEditingFinished: if (d.backend) d.backend.configureDeliverySender(text)
                        }
                    }

                    Flow {
                        Layout.fillWidth: true
                        spacing: 8

                        ActionButton {
                            text: root.deliveryNodeReady ? "Node ready"
                                : (root.deliveryBusy && root.deliveryAction === "create" ? "Starting..." : "Create node")
                            loading: root.deliveryBusy && root.deliveryAction === "create"
                            primary: !root.deliveryNodeReady && !root.deliveryComplete
                            enabled: d.backend && !d.backend.busy && !root.deliveryBusy && !root.deliveryNodeReady
                            onClicked: {
                                root.runDeliveryAction("create", function() { d.backend.deliveryCreateNode() })
                            }
                        }

                        ActionButton {
                            text: root.deliverySubscribed ? "Subscribed"
                                : (root.deliveryBusy && root.deliveryAction === "subscribe" ? "Subscribing..." : "Subscribe")
                            working: root.deliveryBusy && root.deliveryAction === "subscribe"
                            primary: root.deliveryNodeReady && !root.deliverySubscribed && !root.deliveryComplete
                            enabled: d.backend && !d.backend.busy && !root.deliveryBusy && root.deliveryNodeReady && !root.deliverySubscribed
                            onClicked: {
                                root.runDeliveryAction("subscribe", function() { d.backend.deliverySubscribe() })
                            }
                        }

                        ActionButton {
                            text: root.deliveryBusy && root.deliveryAction === "send" ? "Sending..." : "Send proof"
                            loading: root.deliveryBusy && root.deliveryAction === "send"
                            primary: root.proofComplete && !root.deliveryMessageReady && !root.deliveryComplete
                            enabled: d.backend && !d.backend.busy && !root.deliveryBusy && root.proofComplete && root.deliveryNodeReady
                            onClicked: {
                                root.runDeliveryAction("send", function() { d.backend.deliverySendProofMessage() })
                            }
                        }

                        ActionButton {
                            text: d.backend && d.backend.busy && root.runningStep === 5 ? "Verifying..." : "Verify received"
                            primary: root.deliveryReceivedReady && !root.deliveryComplete
                            enabled: d.backend && !d.backend.busy && !root.deliveryBusy && root.deliveryReceivedReady
                            onClicked: {
                                root.startStep(5)
                                d.backend.deliveryVerifyReceivedMessage()
                            }
                        }

                        ActionButton {
                            text: "Clear delivery"
                            enabled: d.backend && !d.backend.busy && !root.deliveryBusy
                            onClicked: d.backend.clearDelivery()
                        }
                    }

                    StatusLine {
                        Layout.fillWidth: true
                        text: root.deliveryComplete
                            ? "Delivery message verified"
                            : (root.deliveryReceivedReady
                                ? "Proof message received. Verify it to finish Delivery."
                                : (root.deliveryMessageReady
                                    ? "Proof message prepared/sent. Use the receiver window to verify inbound delivery."
                                : (d.backend ? root.statusTail(d.backend.deliveryStatus) : "Delivery unavailable"))
                            )
                    }

                    InfoRow {
                        name: "Node"
                        value: root.deliveryNodeReady ? "started" : "not started"
                    }
                    InfoRow {
                        name: "Subscription"
                        value: root.deliverySubscribed ? "subscribed to topic" : "not subscribed"
                    }
                    InfoRow {
                        name: "Peer id"
                        value: d.backend && d.backend.deliveryPeerId.length > 0 ? root.shortValue(d.backend.deliveryPeerId) : "-"
                    }
                    InfoRow {
                        name: "Message"
                        value: root.deliveryReceivedReady ? "received"
                            : (root.deliveryMessageReady ? "prepared/sent" : "-")
                    }
                }
            }

            Rectangle {
                Layout.fillWidth: true
                radius: 8
                color: theme.surface
                border.color: theme.border
                implicitHeight: consoleBody.implicitHeight + 20

                ColumnLayout {
                    id: consoleBody
                    anchors.fill: parent
                    anchors.margins: 10
                    spacing: 8

                    RowLayout {
                        Layout.fillWidth: true

                        Label {
                            Layout.fillWidth: true
                            text: "Console"
                            color: theme.text
                            font.pixelSize: 14
                            font.weight: Font.DemiBold
                        }

                        ActionButton {
                            text: root.showConsole ? "Hide console" : "Show console"
                            enabled: true
                            onClicked: root.showConsole = !root.showConsole
                        }

                        ActionButton {
                            text: "Clear"
                            enabled: d.backend && !d.backend.busy
                            onClicked: d.backend.clearOutputs()
                        }
                    }

                    Rectangle {
                        Layout.fillWidth: true
                        visible: !root.showConsole
                        color: theme.surfaceSoft
                        radius: 6
                        border.color: theme.border
                        implicitHeight: collapsedStatus.implicitHeight + 18

                        Label {
                            id: collapsedStatus
                            anchors.fill: parent
                            anchors.margins: 9
                            text: root.statusTail(d.backend ? d.backend.status : "Backend unavailable")
                            color: theme.muted
                            font.pixelSize: 12
                            elide: Text.ElideRight
                            verticalAlignment: Text.AlignVCenter
                        }
                    }

                    ColumnLayout {
                        Layout.fillWidth: true
                        visible: root.showConsole
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
                            OutputTabButton { text: "Proof" }
                            OutputTabButton { text: "Verify" }
                            OutputTabButton { text: "Gate" }
                            OutputTabButton { text: "Delivery Log" }
                            OutputTabButton { text: "Delivery Msg" }
                            OutputTabButton { text: "Delivery Verify" }
                        }

                        StackLayout {
                            Layout.fillWidth: true
                            Layout.preferredHeight: 320
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

                            OutputPane {
                                autoScroll: true
                                textContent: d.backend ? d.backend.deliveryLog : ""
                            }

                            OutputPane {
                                textContent: d.backend ? d.backend.deliveryMessageJson : ""
                            }

                            OutputPane {
                                textContent: d.backend ? d.backend.deliveryVerifyJson : ""
                            }
                        }
                    }
                }
            }
        }
    }

    component StepNode: Item {
        id: node
        property int index: 0
        property string label: ""
        property string title: ""
        property string detail: ""
        readonly property string state: root.stepState(index)
        readonly property bool selected: root.selectedStep === index
        Layout.preferredWidth: 92
        Layout.preferredHeight: 120

        Rectangle {
            anchors.fill: parent
            radius: 8
            color: node.selected ? (node.state === "failed" ? theme.amberSoft : "#edf7ff") : "transparent"
            border.color: node.selected ? (node.state === "failed" ? "#efd18b" : "#a7d2f1") : "transparent"
        }

        ColumnLayout {
            anchors.fill: parent
            anchors.margins: 5
            spacing: 7

            Item {
                Layout.preferredWidth: 58
                Layout.preferredHeight: 50
                Layout.alignment: Qt.AlignHCenter

                Heartbeat {
                    anchors.centerIn: parent
                    running: node.state === "running"
                    pulseColor: root.stepAccent(node.index)
                    width: 58
                    height: 58
                }

                Rectangle {
                    anchors.centerIn: parent
                    width: 34
                    height: 34
                    radius: 17
                    color: node.state === "completed" ? theme.green
                        : (node.state === "failed" ? theme.amberSoft
                            : (node.state === "pending" ? theme.pending : theme.surface))
                    border.color: root.stepAccent(node.index)
                    border.width: node.selected || node.state === "current" || node.state === "running" || node.state === "failed" ? 3 : 1

                    Label {
                        anchors.centerIn: parent
                        text: node.state === "completed" ? "OK" : (node.state === "failed" ? "!" : String(node.index + 1))
                        color: node.state === "completed" ? "#ffffff"
                            : (node.state === "failed" ? theme.amber
                                : (node.state === "pending" ? "#ffffff" : theme.blue))
                        font.pixelSize: node.state === "completed" ? 10 : 14
                        font.weight: Font.DemiBold
                    }
                }
            }

            Label {
                Layout.fillWidth: true
                text: node.label
                color: theme.faint
                horizontalAlignment: Text.AlignHCenter
                font.pixelSize: 10
                font.letterSpacing: 0
            }

            Label {
                Layout.fillWidth: true
                text: node.title
                color: theme.text
                horizontalAlignment: Text.AlignHCenter
                font.pixelSize: 13
                font.weight: Font.DemiBold
                elide: Text.ElideRight
            }

            Label {
                Layout.fillWidth: true
                Layout.minimumHeight: 18
                text: root.stepStatusText(node.index)
                color: root.stepStatusColor(node.index)
                horizontalAlignment: Text.AlignHCenter
                verticalAlignment: Text.AlignVCenter
                font.pixelSize: 11
                font.weight: Font.Medium
            }
        }

        MouseArea {
            anchors.fill: parent
            cursorShape: Qt.PointingHandCursor
            onClicked: root.selectedStep = node.index
        }
    }

    component GuidedStepCard: Rectangle {
        id: card
        property int stepIndex: 0
        property string title: ""
        property string subtitle: ""
        default property alias content: body.data
        readonly property string state: root.stepState(stepIndex)
        readonly property bool selected: root.selectedStep === stepIndex
        Layout.fillWidth: true
        radius: 8
        color: state === "completed" ? theme.greenSoft
            : (state === "failed" ? theme.amberSoft : theme.surface)
        border.color: selected ? theme.blue : (state === "completed" ? "#9cd8c8"
            : (state === "failed" ? "#efd18b"
                : (state === "running" || state === "current" ? "#9bc9ef" : theme.border))
        )
        border.width: selected ? 2 : 1
        implicitHeight: body.implicitHeight + 28

        RowLayout {
            anchors.fill: parent
            anchors.margins: 14
            spacing: 14

            Item {
                Layout.preferredWidth: 52
                Layout.preferredHeight: 52
                Layout.alignment: Qt.AlignTop

                Heartbeat {
                    anchors.centerIn: parent
                    running: card.state === "running"
                    pulseColor: root.stepAccent(card.stepIndex)
                    width: 52
                    height: 52
                }

                Rectangle {
                    anchors.centerIn: parent
                    width: 34
                    height: 34
                    radius: 17
                    color: card.state === "completed" ? theme.green
                        : (card.state === "failed" ? theme.amberSoft
                            : (card.state === "pending" ? theme.pendingSoft : theme.surface))
                    border.color: root.stepAccent(card.stepIndex)
                    border.width: card.selected || card.state === "current" || card.state === "running" || card.state === "failed" ? 3 : 1

                    Label {
                        anchors.centerIn: parent
                        text: card.state === "completed" ? "OK" : (card.state === "failed" ? "!" : String(card.stepIndex + 1))
                        color: card.state === "completed" ? "#ffffff"
                            : (card.state === "failed" ? theme.amber
                                : (card.state === "pending" ? theme.faint : theme.blue))
                        font.pixelSize: card.state === "completed" ? 10 : 14
                        font.weight: Font.DemiBold
                    }
                }
            }

            ColumnLayout {
                id: body
                Layout.fillWidth: true
                spacing: 10

                RowLayout {
                    Layout.fillWidth: true
                    spacing: 12

                    ColumnLayout {
                        Layout.fillWidth: true
                        spacing: 2

                        Label {
                            text: card.title
                            color: theme.text
                            font.pixelSize: 17
                            font.weight: Font.DemiBold
                        }

                        Label {
                            Layout.fillWidth: true
                            text: card.subtitle
                            color: theme.muted
                            font.pixelSize: 12
                            elide: Text.ElideRight
                        }
                    }

                    StatusPill {
                        value: root.stepStatusText(card.stepIndex)
                        tone: card.state === "completed" ? "green"
                            : (card.state === "failed" ? "amber"
                                : (card.state === "running" || card.state === "current" ? "blue" : "pending"))
                    }
                }
            }
        }
    }

    component DetailsExpander: ColumnLayout {
        id: expander
        property string title: ""
        property bool expanded: false
        default property alias content: detailBody.data
        Layout.fillWidth: true
        spacing: 8

        Button {
            Layout.alignment: Qt.AlignLeft
            implicitHeight: 40
            implicitWidth: Math.max(168, detailLabel.implicitWidth + 34)
            leftPadding: 14
            rightPadding: 14
            topPadding: 0
            bottomPadding: 0
            onClicked: expander.expanded = !expander.expanded
            contentItem: Label {
                id: detailLabel
                text: (expander.expanded ? "Hide " : "Edit ") + expander.title
                color: theme.text
                font.pixelSize: 12
                font.weight: Font.Medium
                horizontalAlignment: Text.AlignHCenter
                verticalAlignment: Text.AlignVCenter
            }
            background: Rectangle {
                radius: 6
                color: theme.surfaceSoft
                border.color: theme.borderStrong
            }
        }

        ColumnLayout {
            id: detailBody
            Layout.fillWidth: true
            visible: expander.expanded
            spacing: 8
        }
    }

    component FieldLabel: Label {
        color: theme.muted
        font.pixelSize: 12
        font.weight: Font.Medium
        Layout.alignment: Qt.AlignVCenter
    }

    component InfoRow: RowLayout {
        property string name: ""
        property string value: ""
        Layout.fillWidth: true
        spacing: 10

        Label {
            Layout.preferredWidth: 118
            text: parent.name
            color: theme.muted
            font.pixelSize: 12
            elide: Text.ElideRight
        }

        Label {
            Layout.fillWidth: true
            text: parent.value.length > 0 ? parent.value : "-"
            color: theme.text
            font.pixelSize: 12
            elide: Text.ElideMiddle
        }
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

    component ActionButton: Button {
        id: control
        property bool primary: false
        property bool loading: false
        property bool working: false
        implicitHeight: 40
        implicitWidth: Math.max(122, label.implicitWidth + 32)
        leftPadding: 14
        rightPadding: 14
        topPadding: 0
        bottomPadding: 0

        contentItem: RowLayout {
            spacing: 8

            Item { Layout.fillWidth: true }

            BusyIndicator {
                visible: control.loading
                running: control.loading
                Layout.preferredWidth: 16
                Layout.preferredHeight: 16
            }

            Label {
                id: label
                text: control.text
                color: (control.loading || control.working) ? "#ffffff" : (!control.enabled ? theme.faint : (control.primary ? "#ffffff" : theme.text))
                horizontalAlignment: Text.AlignHCenter
                verticalAlignment: Text.AlignVCenter
                font.pixelSize: 13
                font.weight: Font.Medium
            }

            Item { Layout.fillWidth: true }
        }

        background: Rectangle {
            radius: 6
            color: (control.loading || control.working) ? theme.blue : (!control.enabled ? "#eef1f4" : (control.primary ? theme.blue : theme.surfaceSoft))
            border.color: (control.loading || control.working) ? theme.blue : (!control.enabled ? theme.border : (control.primary ? theme.blue : theme.borderStrong))
        }
    }

    component StatusPill: Rectangle {
        property string value: ""
        property string tone: "pending"
        radius: 6
        color: tone === "green" ? theme.greenSoft
            : (tone === "blue" ? theme.blueSoft : (tone === "amber" ? theme.amberSoft : theme.pendingSoft))
        border.color: tone === "green" ? "#abd8c8"
            : (tone === "blue" ? "#9bc9ef" : (tone === "amber" ? "#efd18b" : "#c8def1"))
        implicitWidth: pillLabel.implicitWidth + 18
        implicitHeight: pillLabel.implicitHeight + 8

        Label {
            id: pillLabel
            anchors.centerIn: parent
            text: parent.value
            color: parent.tone === "green" ? theme.green
                : (parent.tone === "blue" ? theme.blue : (parent.tone === "amber" ? theme.amber : theme.faint))
            font.pixelSize: 12
            font.weight: Font.Medium
        }
    }

    component StatusLine: Label {
        color: theme.muted
        font.pixelSize: 12
        elide: Text.ElideRight
        verticalAlignment: Text.AlignVCenter
    }

    component Heartbeat: Item {
        id: heartbeat
        property bool running: false
        property color pulseColor: theme.blue
        visible: running

        Rectangle {
            anchors.centerIn: parent
            width: Math.max(38, parent.width * 0.82)
            height: width
            radius: width / 2
            color: "transparent"
            border.color: heartbeat.pulseColor
            border.width: 3
            opacity: heartbeat.running ? 0.48 : 0
            scale: 1

            SequentialAnimation on scale {
                running: heartbeat.running
                loops: Animation.Infinite
                NumberAnimation { from: 0.88; to: 1.13; duration: 620; easing.type: Easing.OutQuad }
                NumberAnimation { from: 1.13; to: 0.88; duration: 360; easing.type: Easing.InQuad }
            }

            SequentialAnimation on opacity {
                running: heartbeat.running
                loops: Animation.Infinite
                NumberAnimation { from: 0.5; to: 0.08; duration: 620; easing.type: Easing.OutQuad }
                NumberAnimation { from: 0.08; to: 0.5; duration: 360; easing.type: Easing.InQuad }
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

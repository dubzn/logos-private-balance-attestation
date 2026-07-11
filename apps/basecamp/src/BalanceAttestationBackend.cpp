#include "BalanceAttestationBackend.h"

#include "logos_api.h"
#include "logos_sdk.h"
#include "logos_types.h"

#include <QCoreApplication>
#include <QDateTime>
#include <QDir>
#include <QFile>
#include <QFileInfo>
#include <QJsonArray>
#include <QJsonDocument>
#include <QJsonObject>
#include <QJsonParseError>
#include <QMap>
#include <QProcessEnvironment>
#include <QRegularExpression>
#include <QSet>
#include <QTimer>

#include <memory>

namespace {

QString repeatByte(const QString &byte)
{
    QString value;
    value.reserve(64);
    for (int i = 0; i < 32; ++i) {
        value += byte;
    }
    return value;
}

QString discoverRepoRoot()
{
    const auto envRoot = qEnvironmentVariable("BALANCE_ATTEST_REPO");
    if (!envRoot.isEmpty()) {
        return QDir::cleanPath(envRoot);
    }

    const auto alternateEnvRoot = qEnvironmentVariable("LOGOS_BALANCE_ATTESTATION_ROOT");
    if (!alternateEnvRoot.isEmpty()) {
        return QDir::cleanPath(alternateEnvRoot);
    }

    const QStringList candidates{
        QDir::currentPath(),
        QCoreApplication::applicationDirPath(),
        QDir::cleanPath(QCoreApplication::applicationDirPath() + "/.."),
        QDir::cleanPath(QCoreApplication::applicationDirPath() + "/../.."),
        QDir::cleanPath(QCoreApplication::applicationDirPath() + "/../../.."),
        QDir::cleanPath(QCoreApplication::applicationDirPath() + "/../../../.."),
    };

    for (const auto &candidate : candidates) {
        if (QFileInfo::exists(candidate + "/Cargo.toml")
            && QFileInfo::exists(candidate + "/scripts/demo-local-sequencer-e2e.sh")) {
            return QDir::cleanPath(candidate);
        }
    }

    return QDir::currentPath();
}

QString defaultLezRepo(const QString &repoRoot)
{
    const auto envLez = qEnvironmentVariable("LOGOS_LEZ_REPO");
    if (!envLez.isEmpty()) {
        return QDir::cleanPath(envLez);
    }

    const auto sibling = QDir::cleanPath(repoRoot + "/../logos-execution-zone");
    if (QFileInfo::exists(sibling + "/Cargo.toml")) {
        return sibling;
    }

    return QDir::cleanPath(QDir::homePath() + "/logos/src/logos-execution-zone");
}

QString tailText(const QString &value, int maxChars = 12000)
{
    if (value.size() <= maxChars) {
        return value;
    }
    return value.right(maxChars);
}

QString stripAnsiSequences(QString value)
{
    static const QRegularExpression ansiPattern(QStringLiteral("\\x1B\\[[0-?]*[ -/]*[@-~]"));
    value.remove(ansiPattern);
    return value;
}

QString barePrivateAccount(QString account)
{
    account = account.trimmed();
    if (account.startsWith("Private/")) {
        return account.mid(QString("Private/").size());
    }
    return account;
}

QString privateAccountDisplay(QString account)
{
    const auto bare = barePrivateAccount(account);
    return bare.isEmpty() ? QString() : QString("Private/") + bare;
}

void addPrivateAccount(
    const QString &label,
    const QString &accountId,
    QSet<QString> &seen,
    QStringList &available
)
{
    const auto bare = barePrivateAccount(accountId);
    if (bare.isEmpty() || seen.contains(bare)) {
        return;
    }

    seen.insert(bare);
    if (label.isEmpty()) {
        available << "- " + privateAccountDisplay(bare);
    } else {
        available << "- " + label + ": " + privateAccountDisplay(bare);
    }
}

void collectLabeledPrivateAccounts(
    const QJsonObject &root,
    QSet<QString> &seen,
    QStringList &available
)
{
    const auto labels = root.value("labels").toObject();
    for (auto it = labels.constBegin(); it != labels.constEnd(); ++it) {
        const auto labelValue = it.value().toObject();
        const auto accountId = labelValue.value("Private").toString();
        addPrivateAccount(it.key(), accountId, seen, available);
    }
}

void collectKeyChainPrivateAccounts(
    const QJsonObject &root,
    QSet<QString> &seen,
    QStringList &available
)
{
    const auto accounts = root.value("key_chain").toObject().value("accounts").toArray();
    for (const auto &accountValue : accounts) {
        const auto privateAccount = accountValue.toObject().value("Private").toObject();
        addPrivateAccount({}, privateAccount.value("account_id").toString(), seen, available);
    }
}

void collectLegacyPrivateAccounts(
    const QJsonObject &root,
    QSet<QString> &seen,
    QStringList &available
)
{
    const auto accounts = root.value("accounts").toArray();
    for (const auto &accountValue : accounts) {
        const auto account = accountValue.toObject();
        if (account.contains("Private")) {
            const auto privateAccount = account.value("Private").toObject();
            addPrivateAccount({}, privateAccount.value("account_id").toString(), seen, available);
        } else if (account.value("kind").toString() == "Private") {
            addPrivateAccount({}, account.value("account_id").toString(), seen, available);
        }
    }
}

} // namespace

BalanceAttestationBackend::BalanceAttestationBackend(QObject *parent)
    : BalanceAttestationSimpleSource(parent)
{
    const auto root = discoverRepoRoot();
    setRepoDir(root);
    setLezRepoDir(defaultLezRepo(root));
    setWalletHomeDir(lezRepoDir() + "/.wallet-local");
    setThreshold("1");
    setChainIdHex(repeatByte("10"));
    setVerifierIdHex(repeatByte("20"));
    setGateIdHex(repeatByte("30"));
    setPresentationChallengeHex(repeatByte("44"));
    setRealProving(true);
    setDeliveryPreset("logos.test");
    setDeliveryMode("Core");
    setDeliveryTopic("/lp0005-balance-attestation/1/proof-envelope/json");
    setDeliveryGroupId("lp0005-balance-gated-chat");
    setDeliverySender("basecamp-presenter");
    setDeliveryRecipient({});
    setDeliveryStatus("Delivery backend waiting for Logos API");
    setStatus("Ready");
}

BalanceAttestationBackend::~BalanceAttestationBackend()
{
    delete m_logos;
}

void BalanceAttestationBackend::initializeLogos(LogosAPI *api)
{
    if (m_logos) {
        return;
    }
    if (!api) {
        setDeliveryStatus("Delivery unavailable: Logos API was not provided");
        appendDeliveryLog("Delivery unavailable: Logos API was not provided");
        return;
    }

    m_logos = new LogosModules(api);
    wireDeliveryEvents();
    setDeliveryStatus("Delivery backend ready");
    appendDeliveryLog("Delivery backend ready");
}

void BalanceAttestationBackend::setRepoDir(QString value)
{
    BalanceAttestationSimpleSource::setRepoDir(QDir::cleanPath(value.trimmed()));
}

void BalanceAttestationBackend::setLezRepoDir(QString value)
{
    const auto cleaned = QDir::cleanPath(value.trimmed());
    const auto previousWallet = walletHomeDir();
    BalanceAttestationSimpleSource::setLezRepoDir(cleaned);

    const auto defaultWallet = cleaned + "/.wallet-local";
    if (previousWallet.isEmpty() || previousWallet.endsWith("/.wallet-local")) {
        setWalletHomeDir(defaultWallet);
    }
}

void BalanceAttestationBackend::setWalletHomeDir(QString value)
{
    BalanceAttestationSimpleSource::setWalletHomeDir(QDir::cleanPath(value.trimmed()));
}

void BalanceAttestationBackend::setPrivateAccount(QString value)
{
    BalanceAttestationSimpleSource::setPrivateAccount(value.trimmed());
}

void BalanceAttestationBackend::setThreshold(QString value)
{
    BalanceAttestationSimpleSource::setThreshold(value.trimmed());
}

void BalanceAttestationBackend::setChainIdHex(QString value)
{
    BalanceAttestationSimpleSource::setChainIdHex(value.trimmed());
}

void BalanceAttestationBackend::setVerifierIdHex(QString value)
{
    BalanceAttestationSimpleSource::setVerifierIdHex(value.trimmed());
}

void BalanceAttestationBackend::setGateIdHex(QString value)
{
    BalanceAttestationSimpleSource::setGateIdHex(value.trimmed());
}

void BalanceAttestationBackend::setPresentationChallengeHex(QString value)
{
    BalanceAttestationSimpleSource::setPresentationChallengeHex(value.trimmed());
}

void BalanceAttestationBackend::setRealProving(bool value)
{
    BalanceAttestationSimpleSource::setRealProving(value);
}

void BalanceAttestationBackend::setDeliveryPreset(QString value)
{
    BalanceAttestationSimpleSource::setDeliveryPreset(value.trimmed());
}

void BalanceAttestationBackend::setDeliveryMode(QString value)
{
    BalanceAttestationSimpleSource::setDeliveryMode(value.trimmed());
}

void BalanceAttestationBackend::setDeliveryTopic(QString value)
{
    BalanceAttestationSimpleSource::setDeliveryTopic(value.trimmed());
}

void BalanceAttestationBackend::setDeliveryGroupId(QString value)
{
    BalanceAttestationSimpleSource::setDeliveryGroupId(value.trimmed());
}

void BalanceAttestationBackend::setDeliverySender(QString value)
{
    BalanceAttestationSimpleSource::setDeliverySender(value.trimmed());
}

void BalanceAttestationBackend::setDeliveryRecipient(QString value)
{
    BalanceAttestationSimpleSource::setDeliveryRecipient(value.trimmed());
}

void BalanceAttestationBackend::configureRepoDir(QString value) { setRepoDir(value); }
void BalanceAttestationBackend::configureLezRepoDir(QString value) { setLezRepoDir(value); }
void BalanceAttestationBackend::configureWalletHomeDir(QString value) { setWalletHomeDir(value); }
void BalanceAttestationBackend::configurePrivateAccount(QString value) { setPrivateAccount(value); }
void BalanceAttestationBackend::configureThreshold(QString value) { setThreshold(value); }
void BalanceAttestationBackend::configureChainIdHex(QString value) { setChainIdHex(value); }
void BalanceAttestationBackend::configureVerifierIdHex(QString value) { setVerifierIdHex(value); }
void BalanceAttestationBackend::configureGateIdHex(QString value) { setGateIdHex(value); }
void BalanceAttestationBackend::configurePresentationChallengeHex(QString value) { setPresentationChallengeHex(value); }
void BalanceAttestationBackend::configureRealProving(bool value) { setRealProving(value); }
void BalanceAttestationBackend::configureDeliveryPreset(QString value) { setDeliveryPreset(value); }
void BalanceAttestationBackend::configureDeliveryMode(QString value) { setDeliveryMode(value); }
void BalanceAttestationBackend::configureDeliveryTopic(QString value) { setDeliveryTopic(value); }
void BalanceAttestationBackend::configureDeliveryGroupId(QString value) { setDeliveryGroupId(value); }
void BalanceAttestationBackend::configureDeliverySender(QString value) { setDeliverySender(value); }
void BalanceAttestationBackend::configureDeliveryRecipient(QString value) { setDeliveryRecipient(value); }

void BalanceAttestationBackend::runPreflight()
{
    runProcess("bash", {scriptPath("check-wallet-preflight.sh")}, {}, OutputTarget::Status);
}

void BalanceAttestationBackend::generateProof()
{
    if (!validateCommonInputs(false, true)) {
        return;
    }

    const auto dir = proofDemoDir();
    setProofRunDir(dir);
    setProofRunJson({});
    setVerifyJson({});
    setGateRunDir({});
    setGateRunJson({});

    runProcess(
        "bash",
        {scriptPath("demo-local-sequencer-e2e.sh")},
        {
            {"PRIVATE_ACCOUNT", normalizedPrivateAccount()},
            {"THRESHOLD", threshold()},
            {"DEMO_DIR", dir},
            {"CHAIN_ID_HEX", chainIdHex()},
            {"VERIFIER_ID_HEX", verifierIdHex()},
            {"GATE_ID_HEX", gateIdHex()},
            {"PRESENTATION_CHALLENGE_HEX", presentationChallengeHex()},
        },
        OutputTarget::ProofRun
    );
}

void BalanceAttestationBackend::verifyEnvelope()
{
    if (!validateCommonInputs(true)) {
        return;
    }

    runProcess(
        "cargo",
        {
            "run",
            "-p",
            "attestation-cli",
            "--",
            "verify",
            "--envelope",
            proofRunDir() + "/envelope.json",
            "--gate",
            proofRunDir() + "/gate.json",
        },
        {},
        OutputTarget::Verify
    );
}

void BalanceAttestationBackend::executeGateAdmit()
{
    if (!validateCommonInputs(true)) {
        return;
    }

    const auto dir = gateDemoDir();
    setGateRunDir(dir);
    setGateRunJson({});

    runProcess(
        "bash",
        {scriptPath("demo-local-gate-e2e.sh")},
        {
            {"RUN_DIR", proofRunDir()},
            {"DEMO_DIR", dir},
            {"REUSE_GATE_ACCOUNTS", "0"},
        },
        OutputTarget::GateRun
    );
}

void BalanceAttestationBackend::deliveryCreateNode()
{
    if (!ensureDeliveryReady(false)) {
        return;
    }
    if (m_deliveryNodeStarted) {
        setDeliveryStatus("Delivery node already started");
        appendDeliveryLog("Delivery node already started");
        return;
    }

    QJsonObject cfg{
        {"logLevel", "INFO"},
        {"mode", deliveryMode().isEmpty() ? QString("Core") : deliveryMode()},
        {"preset", deliveryPreset().isEmpty() ? QString("logos.test") : deliveryPreset()},
    };
    const auto cfgJson = QString::fromUtf8(QJsonDocument(cfg).toJson(QJsonDocument::Compact));
    appendDeliveryLog("createNode " + cfgJson);

    LogosResult created = m_logos->delivery_module.createNode(cfgJson);
    if (!created.success) {
        const auto error = "createNode failed: " + created.getError();
        setDeliveryStatus(error);
        appendDeliveryLog(error);
        return;
    }

    LogosResult started = m_logos->delivery_module.start();
    if (!started.success) {
        const auto error = "start failed: " + started.getError();
        setDeliveryStatus(error);
        appendDeliveryLog(error);
        return;
    }

    setDeliveryStatus("Delivery node started");
    appendDeliveryLog("Delivery node started");
    m_deliveryNodeStarted = true;

    LogosResult version = m_logos->delivery_module.getNodeInfo(QStringLiteral("Version"));
    if (version.success) {
        setDeliveryVersion(version.getString());
    }

    if (!m_deliveryPollTimer) {
        m_deliveryPollTimer = new QTimer(this);
        m_deliveryPollTimer->setInterval(3000);
        connect(m_deliveryPollTimer, &QTimer::timeout, this, &BalanceAttestationBackend::refreshDeliveryPeerId);
    }
    refreshDeliveryPeerId();
    m_deliveryPollTimer->start();
}

void BalanceAttestationBackend::deliverySubscribe()
{
    if (!ensureDeliveryReady(false)) {
        return;
    }
    if (deliveryTopic().isEmpty()) {
        setDeliveryStatus("Delivery topic is required");
        return;
    }

    LogosResult result = m_logos->delivery_module.subscribe(deliveryTopic());
    if (!result.success) {
        const auto error = "subscribe failed: " + result.getError();
        setDeliveryStatus(error);
        appendDeliveryLog(error);
        return;
    }
    setDeliveryStatus("Subscribed to " + deliveryTopic());
    appendDeliveryLog("Subscribed to " + deliveryTopic());
}

void BalanceAttestationBackend::deliverySendProofMessage()
{
    if (!ensureDeliveryReady(true)) {
        return;
    }
    if (deliveryTopic().isEmpty() || deliveryGroupId().isEmpty() || deliverySender().isEmpty()) {
        setDeliveryStatus("Delivery topic, group id, and sender are required");
        return;
    }

    const auto dir = deliveryRunDir().isEmpty() ? deliveryDemoDir() : deliveryRunDir();
    setDeliveryRunDir(dir);

    QString messageJson;
    const auto path = deliveryMessagePath();
    if (!writeDeliveryMessageFile(path, &messageJson)) {
        return;
    }

    LogosResult result = m_logos->delivery_module.send(deliveryTopic(), messageJson.toUtf8());
    if (!result.success) {
        const auto error = "send failed: " + result.getError();
        setDeliveryStatus(error);
        appendDeliveryLog(error);
        return;
    }

    const auto requestId = result.getString();
    setDeliveryStatus("Sent proof message: " + requestId);
    appendDeliveryLog("Sent proof message request_id=" + requestId);
}

void BalanceAttestationBackend::deliveryVerifyReceivedMessage()
{
    if (!validateCommonInputs(false)) {
        return;
    }
    const auto dir = deliveryRunDir().isEmpty() ? deliveryDemoDir() : deliveryRunDir();
    setDeliveryRunDir(dir);

    const auto messagePath = deliveryMessagePath();
    if (!QFileInfo::exists(messagePath)) {
        setDeliveryStatus("No received Delivery proof message is available");
        return;
    }
    if (!writeDeliveryGateFile(deliveryGatePath())) {
        return;
    }

    runProcess(
        "cargo",
        {
            "run",
            "-p",
            "attestation-cli",
            "--",
            "message-verify",
            "--message",
            messagePath,
            "--gate",
            deliveryGatePath(),
        },
        {},
        OutputTarget::DeliveryVerify
    );
}

void BalanceAttestationBackend::clearOutputs()
{
    setProofRunDir({});
    setGateRunDir({});
    setProofRunJson({});
    setVerifyJson({});
    setGateRunJson({});
    setStatus("Ready");
}

void BalanceAttestationBackend::clearDelivery()
{
    setDeliveryRunDir({});
    setDeliveryMessageJson({});
    setDeliveryVerifyJson({});
    setDeliveryLog({});
    setDeliveryStatus(m_logos ? QString("Delivery backend ready") : QString("Delivery backend waiting for Logos API"));
}

void BalanceAttestationBackend::setBusy(bool value)
{
    BalanceAttestationSimpleSource::setBusy(value);
}

void BalanceAttestationBackend::setStatus(QString value)
{
    const auto cleaned = stripAnsiSequences(value).trimmed();
    BalanceAttestationSimpleSource::setStatus(tailText(cleaned.isEmpty() ? QString("Done") : cleaned));
}

void BalanceAttestationBackend::setProofRunDir(QString value)
{
    BalanceAttestationSimpleSource::setProofRunDir(value);
}

void BalanceAttestationBackend::setGateRunDir(QString value)
{
    BalanceAttestationSimpleSource::setGateRunDir(value);
}

void BalanceAttestationBackend::setProofRunJson(QString value)
{
    BalanceAttestationSimpleSource::setProofRunJson(value);
}

void BalanceAttestationBackend::setVerifyJson(QString value)
{
    BalanceAttestationSimpleSource::setVerifyJson(value);
}

void BalanceAttestationBackend::setGateRunJson(QString value)
{
    BalanceAttestationSimpleSource::setGateRunJson(value);
}

void BalanceAttestationBackend::setDeliveryStatus(QString value)
{
    BalanceAttestationSimpleSource::setDeliveryStatus(tailText(value.trimmed(), 2000));
}

void BalanceAttestationBackend::setDeliveryPeerId(QString value)
{
    BalanceAttestationSimpleSource::setDeliveryPeerId(value.trimmed());
}

void BalanceAttestationBackend::setDeliveryVersion(QString value)
{
    BalanceAttestationSimpleSource::setDeliveryVersion(value.trimmed());
}

void BalanceAttestationBackend::setDeliveryRunDir(QString value)
{
    BalanceAttestationSimpleSource::setDeliveryRunDir(QDir::cleanPath(value.trimmed()));
}

void BalanceAttestationBackend::setDeliveryMessageJson(QString value)
{
    BalanceAttestationSimpleSource::setDeliveryMessageJson(tailText(value.trimmed(), 24000));
}

void BalanceAttestationBackend::setDeliveryVerifyJson(QString value)
{
    BalanceAttestationSimpleSource::setDeliveryVerifyJson(tailText(value.trimmed(), 12000));
}

void BalanceAttestationBackend::setDeliveryLog(QString value)
{
    BalanceAttestationSimpleSource::setDeliveryLog(tailText(value.trimmed(), 12000));
}

bool BalanceAttestationBackend::validateCommonInputs(bool requireProofRun, bool requireWalletAccount)
{
    if (repoDir().isEmpty() || !QFileInfo::exists(repoDir() + "/Cargo.toml")) {
        setStatus("Repository directory is invalid");
        return false;
    }
    if (lezRepoDir().isEmpty() || !QFileInfo::exists(lezRepoDir() + "/Cargo.toml")) {
        setStatus("LEZ repository directory is invalid");
        return false;
    }
    if (walletHomeDir().isEmpty()) {
        setStatus("Wallet home is required");
        return false;
    }
    if (privateAccount().isEmpty()) {
        setStatus("Private account is required");
        return false;
    }
    bool thresholdOk = false;
    threshold().toULongLong(&thresholdOk);
    if (!thresholdOk) {
        setStatus("Threshold must be a decimal integer");
        return false;
    }
    if (requireProofRun && (proofRunDir().isEmpty() || !QFileInfo::exists(proofRunDir() + "/envelope.json"))) {
        setStatus("Generate a proof before this action");
        return false;
    }
    if (requireWalletAccount && !validatePrivateAccountInWallet()) {
        return false;
    }
    return true;
}

bool BalanceAttestationBackend::validatePrivateAccountInWallet()
{
    const auto selected = barePrivateAccount(privateAccount());
    if (selected.isEmpty()) {
        setStatus("Private account is required");
        return false;
    }

    const auto storagePath = QDir::cleanPath(walletHomeDir() + "/storage.json");
    QFile storageFile(storagePath);
    if (!storageFile.exists()) {
        setStatus(
            "Wallet storage was not found.\n\n"
            "Wallet home:\n" + walletHomeDir()
            + "\n\nCreate or select a wallet home first, then run:\n"
              "wallet account new private --label private-balance"
        );
        return false;
    }

    if (!storageFile.open(QIODevice::ReadOnly | QIODevice::Text)) {
        setStatus("Could not read wallet storage:\n" + storagePath);
        return false;
    }

    QJsonParseError parseError;
    const auto document = QJsonDocument::fromJson(storageFile.readAll(), &parseError);
    if (parseError.error != QJsonParseError::NoError || !document.isObject()) {
        setStatus(
            "Wallet storage is not valid JSON for this Basecamp check.\n\n"
            "Path:\n" + storagePath
            + "\n\nParse error:\n" + parseError.errorString()
        );
        return false;
    }

    const auto root = document.object();
    QSet<QString> seen;
    QStringList available;
    collectLabeledPrivateAccounts(root, seen, available);
    collectKeyChainPrivateAccounts(root, seen, available);
    collectLegacyPrivateAccounts(root, seen, available);

    if (seen.contains(selected)) {
        return true;
    }

    QString message =
        "Private account not found in the selected wallet home.\n\n"
        "Selected account:\n" + privateAccountDisplay(selected)
        + "\n\nWallet home:\n" + walletHomeDir();

    if (available.isEmpty()) {
        message +=
            "\n\nNo private accounts were found in this wallet storage.\n\n"
            "Create one from the matching LEZ checkout:\n"
            "wallet account new private --label private-balance";
    } else {
        message += "\n\nAvailable private accounts:\n" + available.join("\n");
    }

    message +=
        "\n\nUse one of those accounts in the Private account field, or switch "
        "Wallet home to the directory that owns the selected account.";

    setStatus(message);
    return false;
}

QString BalanceAttestationBackend::normalizedPrivateAccount() const
{
    auto account = privateAccount().trimmed();
    if (!account.startsWith("Private/")) {
        account = "Private/" + account;
    }
    return account;
}

QString BalanceAttestationBackend::proofDemoDir() const
{
    return QDir::cleanPath(repoDir() + "/.demo-runs/basecamp/" + timestamp() + "/proof");
}

QString BalanceAttestationBackend::gateDemoDir() const
{
    const auto base = proofRunDir();
    if (base.endsWith("/proof")) {
        return QDir::cleanPath(base.left(base.size() - QString("/proof").size()) + "/gate");
    }
    return QDir::cleanPath(repoDir() + "/.demo-runs/basecamp/" + timestamp() + "/gate");
}

QString BalanceAttestationBackend::deliveryDemoDir() const
{
    const auto base = proofRunDir();
    if (base.endsWith("/proof")) {
        return QDir::cleanPath(base.left(base.size() - QString("/proof").size()) + "/delivery");
    }
    return QDir::cleanPath(repoDir() + "/.demo-runs/basecamp-delivery/" + timestamp());
}

QString BalanceAttestationBackend::scriptPath(const QString &name) const
{
    return QDir::cleanPath(repoDir() + "/scripts/" + name);
}

QString BalanceAttestationBackend::readTextFile(const QString &path) const
{
    QFile file(path);
    if (!file.open(QIODevice::ReadOnly | QIODevice::Text)) {
        return {};
    }
    return QString::fromUtf8(file.readAll());
}

QString BalanceAttestationBackend::timestamp() const
{
    return QDateTime::currentDateTimeUtc().toString("yyyyMMdd'T'hhmmss'Z'");
}

QString BalanceAttestationBackend::deliveryMessagePath() const
{
    return QDir::cleanPath(deliveryRunDir() + "/proof-message.json");
}

QString BalanceAttestationBackend::deliveryGatePath() const
{
    return QDir::cleanPath(deliveryRunDir() + "/gate.json");
}

bool BalanceAttestationBackend::writeDeliveryGateFile(const QString &path)
{
    QJsonObject gate{
        {"chain_id", chainIdHex()},
        {"verifier_id", verifierIdHex()},
        {"gate_id", gateIdHex()},
        {"presentation_challenge", presentationChallengeHex()},
        {"threshold", threshold()},
    };

    QDir().mkpath(QFileInfo(path).absolutePath());
    QFile file(path);
    if (!file.open(QIODevice::WriteOnly | QIODevice::Text)) {
        setDeliveryStatus("Could not write Delivery gate file:\n" + path);
        return false;
    }
    file.write(QJsonDocument(gate).toJson(QJsonDocument::Indented));
    return true;
}

bool BalanceAttestationBackend::writeDeliveryMessageFile(const QString &path, QString *messageJson)
{
    const auto envelopePath = proofRunDir() + "/envelope.json";
    QFile envelopeFile(envelopePath);
    if (!envelopeFile.open(QIODevice::ReadOnly | QIODevice::Text)) {
        setDeliveryStatus("Could not read proof envelope:\n" + envelopePath);
        return false;
    }

    QJsonParseError parseError;
    const auto envelopeDoc = QJsonDocument::fromJson(envelopeFile.readAll(), &parseError);
    if (parseError.error != QJsonParseError::NoError || !envelopeDoc.isObject()) {
        setDeliveryStatus("Proof envelope is not valid JSON:\n" + parseError.errorString());
        return false;
    }

    QJsonValue recipientValue(QJsonValue::Null);
    if (!deliveryRecipient().isEmpty()) {
        recipientValue = deliveryRecipient();
    }

    QJsonObject message{
        {"version", 1},
        {"transport", "logos-messaging"},
        {"message_id", "basecamp-delivery-" + timestamp()},
        {"group_id", deliveryGroupId()},
        {"sender", deliverySender()},
        {"recipient", recipientValue},
        {"created_at_unix", QDateTime::currentSecsSinceEpoch()},
        {"envelope", envelopeDoc.object()},
    };

    const auto json = QString::fromUtf8(QJsonDocument(message).toJson(QJsonDocument::Indented));
    QDir().mkpath(QFileInfo(path).absolutePath());
    QFile file(path);
    if (!file.open(QIODevice::WriteOnly | QIODevice::Text)) {
        setDeliveryStatus("Could not write Delivery proof message:\n" + path);
        return false;
    }
    file.write(json.toUtf8());
    setDeliveryMessageJson(json);
    if (messageJson) {
        *messageJson = json;
    }
    appendDeliveryLog("Prepared proof message " + path);
    return true;
}

bool BalanceAttestationBackend::ensureDeliveryReady(bool requireProofRun)
{
    if (!m_logos) {
        setDeliveryStatus("Delivery unavailable: Basecamp did not provide Logos API");
        return false;
    }
    if (requireProofRun && (proofRunDir().isEmpty() || !QFileInfo::exists(proofRunDir() + "/envelope.json"))) {
        setDeliveryStatus("Generate a proof before sending a Delivery proof message");
        return false;
    }
    return true;
}

void BalanceAttestationBackend::appendDeliveryLog(const QString &line)
{
    const auto stamp = QDateTime::currentDateTime().toString("hh:mm:ss");
    const auto next = (deliveryLog().isEmpty() ? QString() : deliveryLog() + "\n")
        + "[" + stamp + "] " + line;
    setDeliveryLog(next);
}

void BalanceAttestationBackend::wireDeliveryEvents()
{
    if (!m_logos) {
        return;
    }

    m_logos->delivery_module.on("connectionStateChanged", [this](const QVariantList &data) {
        if (data.isEmpty()) {
            return;
        }
        const auto state = data.at(0).toString();
        setDeliveryStatus(state);
        appendDeliveryLog("connectionStateChanged " + state);
    });

    m_logos->delivery_module.on("messageReceived", [this](const QVariantList &data) {
        if (data.size() < 4) {
            return;
        }
        const auto topic = data.at(1).toString();
        const QByteArray payload = data.at(2).toByteArray();
        const auto hash = data.at(0).toString();
        const auto timestampNs = data.at(3).toLongLong();

        const auto dir = deliveryRunDir().isEmpty() ? deliveryDemoDir() : deliveryRunDir();
        setDeliveryRunDir(dir);
        QDir().mkpath(dir);

        const auto path = deliveryMessagePath();
        QFile file(path);
        if (file.open(QIODevice::WriteOnly | QIODevice::Text)) {
            file.write(payload);
        }
        setDeliveryMessageJson(QString::fromUtf8(payload));
        setDeliveryStatus("Received proof message on " + topic);
        appendDeliveryLog("messageReceived topic=" + topic + " hash=" + hash);
        emit deliveryMessageReceived(topic, hash, timestampNs);
    });

    m_logos->delivery_module.on("messageSent", [this](const QVariantList &data) {
        if (data.size() < 3) {
            return;
        }
        appendDeliveryLog("messageSent request_id=" + data.at(0).toString() + " hash=" + data.at(1).toString());
        emit deliveryMessageSent(data.at(0).toString(), data.at(1).toString(), data.at(2).toLongLong());
    });

    m_logos->delivery_module.on("messagePropagated", [this](const QVariantList &data) {
        if (data.size() < 3) {
            return;
        }
        appendDeliveryLog("messagePropagated request_id=" + data.at(0).toString() + " hash=" + data.at(1).toString());
        emit deliveryMessagePropagated(data.at(0).toString(), data.at(1).toString(), data.at(2).toLongLong());
    });

    m_logos->delivery_module.on("messageError", [this](const QVariantList &data) {
        if (data.size() < 4) {
            return;
        }
        const auto error = data.at(2).toString();
        setDeliveryStatus("Delivery message error: " + error);
        appendDeliveryLog("messageError request_id=" + data.at(0).toString() + " error=" + error);
        emit deliveryMessageError(data.at(0).toString(), data.at(1).toString(), error, data.at(3).toLongLong());
    });
}

void BalanceAttestationBackend::refreshDeliveryPeerId()
{
    if (!m_logos) {
        return;
    }
    LogosResult peer = m_logos->delivery_module.getNodeInfo(QStringLiteral("MyPeerId"));
    if (peer.success) {
        setDeliveryPeerId(peer.getString());
    }
}

QProcessEnvironment BalanceAttestationBackend::processEnvironment(const QMap<QString, QString> &overrides) const
{
    auto env = QProcessEnvironment::systemEnvironment();
    env.insert("BALANCE_ATTEST_REPO", repoDir());
    env.insert("LOGOS_BALANCE_ATTESTATION_ROOT", repoDir());
    env.insert("LOGOS_LEZ_REPO", lezRepoDir());
    env.insert("LEZ_REPO", lezRepoDir());
    env.insert("NSSA_WALLET_HOME_DIR", walletHomeDir());
    env.insert("LEE_WALLET_HOME_DIR", walletHomeDir());
    env.insert("RISC0_DEV_MODE", realProving() ? "0" : "1");

    const auto recursionZkr = QDir::cleanPath(repoDir() + "/.risc0-cache/recursion_zkr.zip");
    if (QFileInfo::exists(recursionZkr)) {
        env.insert("RECURSION_SRC_PATH", recursionZkr);
    }

    for (auto it = overrides.constBegin(); it != overrides.constEnd(); ++it) {
        env.insert(it.key(), it.value());
    }
    return env;
}

void BalanceAttestationBackend::runProcess(
    const QString &program,
    const QStringList &arguments,
    const QMap<QString, QString> &envOverrides,
    OutputTarget outputTarget
)
{
    if (busy()) {
        setStatus("Another command is already running");
        return;
    }

    setBusy(true);
    setStatus("Running:\n" + program + " " + arguments.join(" "));

    auto *process = new QProcess(this);
    auto stdoutBuffer = std::make_shared<QString>();
    auto stderrBuffer = std::make_shared<QString>();
    process->setWorkingDirectory(repoDir());
    process->setProcessEnvironment(processEnvironment(envOverrides));

    auto appendOutput = [this, stdoutBuffer, stderrBuffer](const QString &text, bool isStdErr) {
        if (text.isEmpty()) {
            return;
        }
        if (isStdErr) {
            stderrBuffer->append(text);
        } else {
            stdoutBuffer->append(text);
        }

        const auto combined = QString(*stdoutBuffer
            + (stderrBuffer->isEmpty() ? QString() : "\n" + *stderrBuffer)).trimmed();
        if (!combined.isEmpty()) {
            setStatus(combined);
        }
    };

    connect(process, &QProcess::readyReadStandardOutput, this, [process, appendOutput]() {
        appendOutput(QString::fromUtf8(process->readAllStandardOutput()), false);
    });
    connect(process, &QProcess::readyReadStandardError, this, [process, appendOutput]() {
        appendOutput(QString::fromUtf8(process->readAllStandardError()), true);
    });

    connect(process, QOverload<int, QProcess::ExitStatus>::of(&QProcess::finished), this, [this, process, outputTarget, stdoutBuffer, stderrBuffer](int exitCode, QProcess::ExitStatus exitStatus) {
        stdoutBuffer->append(QString::fromUtf8(process->readAllStandardOutput()));
        stderrBuffer->append(QString::fromUtf8(process->readAllStandardError()));

        const auto combined = QString(*stdoutBuffer
            + (stderrBuffer->isEmpty() ? QString() : "\n" + *stderrBuffer)).trimmed();

        if (exitStatus == QProcess::NormalExit && exitCode == 0) {
            if (outputTarget == OutputTarget::ProofRun) {
                setProofRunJson(readTextFile(proofRunDir() + "/run.json"));
                setVerifyJson(readTextFile(proofRunDir() + "/verify.json"));
            } else if (outputTarget == OutputTarget::Verify) {
                setVerifyJson(stdoutBuffer->trimmed());
            } else if (outputTarget == OutputTarget::GateRun) {
                setGateRunJson(readTextFile(gateRunDir() + "/run.json"));
            } else if (outputTarget == OutputTarget::DeliveryVerify) {
                setDeliveryVerifyJson(stdoutBuffer->trimmed());
                setDeliveryStatus("Received Delivery proof message verified");
                appendDeliveryLog("message-verify ok");
            }
            setStatus(combined.isEmpty() ? QString("Done") : combined);
        } else {
            setStatus(combined.isEmpty() ? QString("Command failed") : combined);
            if (outputTarget == OutputTarget::DeliveryVerify) {
                setDeliveryVerifyJson(combined);
                setDeliveryStatus("Received Delivery proof message rejected");
                appendDeliveryLog("message-verify failed");
            }
        }

        setBusy(false);
        process->deleteLater();
    });

    process->start(program, arguments);
}

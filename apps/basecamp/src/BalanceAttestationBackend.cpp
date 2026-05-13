#include "BalanceAttestationBackend.h"

#include <QCoreApplication>
#include <QDateTime>
#include <QDir>
#include <QFile>
#include <QFileInfo>
#include <QMap>
#include <QProcessEnvironment>

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

} // namespace

BalanceAttestationBackend::BalanceAttestationBackend(QObject *parent)
    : QObject(parent),
      m_repoDir(discoverRepoRoot()),
      m_lezRepoDir(defaultLezRepo(m_repoDir)),
      m_walletHomeDir(m_lezRepoDir + "/.wallet-local"),
      m_chainIdHex(repeatByte("10")),
      m_verifierIdHex(repeatByte("20")),
      m_gateIdHex(repeatByte("30")),
      m_presentationChallengeHex(repeatByte("44"))
{
}

QString BalanceAttestationBackend::repoDir() const { return m_repoDir; }
QString BalanceAttestationBackend::lezRepoDir() const { return m_lezRepoDir; }
QString BalanceAttestationBackend::walletHomeDir() const { return m_walletHomeDir; }
QString BalanceAttestationBackend::privateAccount() const { return m_privateAccount; }
QString BalanceAttestationBackend::threshold() const { return m_threshold; }
QString BalanceAttestationBackend::chainIdHex() const { return m_chainIdHex; }
QString BalanceAttestationBackend::verifierIdHex() const { return m_verifierIdHex; }
QString BalanceAttestationBackend::gateIdHex() const { return m_gateIdHex; }
QString BalanceAttestationBackend::presentationChallengeHex() const { return m_presentationChallengeHex; }
bool BalanceAttestationBackend::realProving() const { return m_realProving; }
bool BalanceAttestationBackend::busy() const { return m_busy; }
QString BalanceAttestationBackend::status() const { return m_status; }
QString BalanceAttestationBackend::proofRunDir() const { return m_proofRunDir; }
QString BalanceAttestationBackend::gateRunDir() const { return m_gateRunDir; }
QString BalanceAttestationBackend::proofRunJson() const { return m_proofRunJson; }
QString BalanceAttestationBackend::verifyJson() const { return m_verifyJson; }
QString BalanceAttestationBackend::gateRunJson() const { return m_gateRunJson; }

void BalanceAttestationBackend::configureRepoDir(const QString &value)
{
    const auto cleaned = QDir::cleanPath(value.trimmed());
    if (m_repoDir == cleaned) {
        return;
    }
    m_repoDir = cleaned;
    emit repoDirChanged();
}

void BalanceAttestationBackend::configureLezRepoDir(const QString &value)
{
    const auto cleaned = QDir::cleanPath(value.trimmed());
    if (m_lezRepoDir == cleaned) {
        return;
    }
    m_lezRepoDir = cleaned;
    emit lezRepoDirChanged();

    const auto defaultWallet = cleaned + "/.wallet-local";
    if (m_walletHomeDir.isEmpty() || m_walletHomeDir.endsWith("/.wallet-local")) {
        m_walletHomeDir = defaultWallet;
        emit walletHomeDirChanged();
    }
}

void BalanceAttestationBackend::configureWalletHomeDir(const QString &value)
{
    const auto cleaned = QDir::cleanPath(value.trimmed());
    if (m_walletHomeDir == cleaned) {
        return;
    }
    m_walletHomeDir = cleaned;
    emit walletHomeDirChanged();
}

void BalanceAttestationBackend::configurePrivateAccount(const QString &value)
{
    const auto trimmed = value.trimmed();
    if (m_privateAccount == trimmed) {
        return;
    }
    m_privateAccount = trimmed;
    emit privateAccountChanged();
}

void BalanceAttestationBackend::configureThreshold(const QString &value)
{
    const auto trimmed = value.trimmed();
    if (m_threshold == trimmed) {
        return;
    }
    m_threshold = trimmed;
    emit thresholdChanged();
}

void BalanceAttestationBackend::configureChainIdHex(const QString &value)
{
    const auto trimmed = value.trimmed();
    if (m_chainIdHex == trimmed) {
        return;
    }
    m_chainIdHex = trimmed;
    emit chainIdHexChanged();
}

void BalanceAttestationBackend::configureVerifierIdHex(const QString &value)
{
    const auto trimmed = value.trimmed();
    if (m_verifierIdHex == trimmed) {
        return;
    }
    m_verifierIdHex = trimmed;
    emit verifierIdHexChanged();
}

void BalanceAttestationBackend::configureGateIdHex(const QString &value)
{
    const auto trimmed = value.trimmed();
    if (m_gateIdHex == trimmed) {
        return;
    }
    m_gateIdHex = trimmed;
    emit gateIdHexChanged();
}

void BalanceAttestationBackend::configurePresentationChallengeHex(const QString &value)
{
    const auto trimmed = value.trimmed();
    if (m_presentationChallengeHex == trimmed) {
        return;
    }
    m_presentationChallengeHex = trimmed;
    emit presentationChallengeHexChanged();
}

void BalanceAttestationBackend::configureRealProving(bool value)
{
    if (m_realProving == value) {
        return;
    }
    m_realProving = value;
    emit realProvingChanged();
}

void BalanceAttestationBackend::runPreflight()
{
    runProcess("bash", {scriptPath("check-wallet-preflight.sh")}, {}, OutputTarget::Status);
}

void BalanceAttestationBackend::generateProof()
{
    if (!validateCommonInputs(false)) {
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
            {"THRESHOLD", m_threshold},
            {"DEMO_DIR", dir},
            {"CHAIN_ID_HEX", m_chainIdHex},
            {"VERIFIER_ID_HEX", m_verifierIdHex},
            {"GATE_ID_HEX", m_gateIdHex},
            {"PRESENTATION_CHALLENGE_HEX", m_presentationChallengeHex},
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
            m_proofRunDir + "/envelope.json",
            "--gate",
            m_proofRunDir + "/gate.json",
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
            {"RUN_DIR", m_proofRunDir},
            {"DEMO_DIR", dir},
            {"REUSE_GATE_ACCOUNTS", "0"},
        },
        OutputTarget::GateRun
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

void BalanceAttestationBackend::setBusy(bool value)
{
    if (m_busy == value) {
        return;
    }
    m_busy = value;
    emit busyChanged();
}

void BalanceAttestationBackend::setStatus(const QString &value)
{
    m_status = tailText(value.trimmed().isEmpty() ? QString("Done") : value.trimmed());
    emit statusChanged();
}

void BalanceAttestationBackend::setProofRunDir(const QString &value)
{
    if (m_proofRunDir == value) {
        return;
    }
    m_proofRunDir = value;
    emit proofRunDirChanged();
}

void BalanceAttestationBackend::setGateRunDir(const QString &value)
{
    if (m_gateRunDir == value) {
        return;
    }
    m_gateRunDir = value;
    emit gateRunDirChanged();
}

void BalanceAttestationBackend::setProofRunJson(const QString &value)
{
    m_proofRunJson = value;
    emit proofRunJsonChanged();
}

void BalanceAttestationBackend::setVerifyJson(const QString &value)
{
    m_verifyJson = value;
    emit verifyJsonChanged();
}

void BalanceAttestationBackend::setGateRunJson(const QString &value)
{
    m_gateRunJson = value;
    emit gateRunJsonChanged();
}

bool BalanceAttestationBackend::validateCommonInputs(bool requireProofRun)
{
    if (m_repoDir.isEmpty() || !QFileInfo::exists(m_repoDir + "/Cargo.toml")) {
        setStatus("Repository directory is invalid");
        return false;
    }
    if (m_lezRepoDir.isEmpty() || !QFileInfo::exists(m_lezRepoDir + "/Cargo.toml")) {
        setStatus("LEZ repository directory is invalid");
        return false;
    }
    if (m_walletHomeDir.isEmpty()) {
        setStatus("Wallet home is required");
        return false;
    }
    if (m_privateAccount.isEmpty()) {
        setStatus("Private account is required");
        return false;
    }
    bool thresholdOk = false;
    m_threshold.toULongLong(&thresholdOk);
    if (!thresholdOk) {
        setStatus("Threshold must be a decimal integer");
        return false;
    }
    if (requireProofRun && (m_proofRunDir.isEmpty() || !QFileInfo::exists(m_proofRunDir + "/envelope.json"))) {
        setStatus("Generate a proof before this action");
        return false;
    }
    return true;
}

QString BalanceAttestationBackend::normalizedPrivateAccount() const
{
    auto account = m_privateAccount.trimmed();
    if (!account.startsWith("Private/")) {
        account = "Private/" + account;
    }
    return account;
}

QString BalanceAttestationBackend::proofDemoDir() const
{
    return QDir::cleanPath(m_repoDir + "/.demo-runs/basecamp/" + timestamp() + "/proof");
}

QString BalanceAttestationBackend::gateDemoDir() const
{
    const auto base = m_proofRunDir;
    if (base.endsWith("/proof")) {
        return QDir::cleanPath(base.left(base.size() - QString("/proof").size()) + "/gate");
    }
    return QDir::cleanPath(m_repoDir + "/.demo-runs/basecamp/" + timestamp() + "/gate");
}

QString BalanceAttestationBackend::scriptPath(const QString &name) const
{
    return QDir::cleanPath(m_repoDir + "/scripts/" + name);
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

QProcessEnvironment BalanceAttestationBackend::processEnvironment(const QMap<QString, QString> &overrides) const
{
    auto env = QProcessEnvironment::systemEnvironment();
    env.insert("BALANCE_ATTEST_REPO", m_repoDir);
    env.insert("LOGOS_BALANCE_ATTESTATION_ROOT", m_repoDir);
    env.insert("LOGOS_LEZ_REPO", m_lezRepoDir);
    env.insert("LEZ_REPO", m_lezRepoDir);
    env.insert("NSSA_WALLET_HOME_DIR", m_walletHomeDir);
    env.insert("RISC0_DEV_MODE", m_realProving ? "0" : "1");
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
    if (m_busy) {
        setStatus("Another command is already running");
        return;
    }

    setBusy(true);
    setStatus(program + " " + arguments.join(" "));

    auto *process = new QProcess(this);
    process->setWorkingDirectory(m_repoDir);
    process->setProcessEnvironment(processEnvironment(envOverrides));

    connect(process, QOverload<int, QProcess::ExitStatus>::of(&QProcess::finished), this, [this, process, outputTarget](int exitCode, QProcess::ExitStatus exitStatus) {
        const auto stdoutText = QString::fromUtf8(process->readAllStandardOutput());
        const auto stderrText = QString::fromUtf8(process->readAllStandardError());
        const auto combined = QString(stdoutText + (stderrText.isEmpty() ? QString() : "\n" + stderrText)).trimmed();

        if (exitStatus == QProcess::NormalExit && exitCode == 0) {
            if (outputTarget == OutputTarget::ProofRun) {
                setProofRunJson(readTextFile(m_proofRunDir + "/run.json"));
                setVerifyJson(readTextFile(m_proofRunDir + "/verify.json"));
            } else if (outputTarget == OutputTarget::Verify) {
                setVerifyJson(stdoutText.trimmed());
            } else if (outputTarget == OutputTarget::GateRun) {
                setGateRunJson(readTextFile(m_gateRunDir + "/run.json"));
            }
            setStatus(combined.isEmpty() ? QString("Done") : combined);
        } else {
            setStatus(combined.isEmpty() ? QString("Command failed") : combined);
        }

        setBusy(false);
        process->deleteLater();
    });

    process->start(program, arguments);
}

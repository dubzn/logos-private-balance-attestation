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
    setStatus("Ready");
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
    BalanceAttestationSimpleSource::setBusy(value);
}

void BalanceAttestationBackend::setStatus(QString value)
{
    BalanceAttestationSimpleSource::setStatus(tailText(value.trimmed().isEmpty() ? QString("Done") : value.trimmed()));
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

bool BalanceAttestationBackend::validateCommonInputs(bool requireProofRun)
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
    return true;
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

QProcessEnvironment BalanceAttestationBackend::processEnvironment(const QMap<QString, QString> &overrides) const
{
    auto env = QProcessEnvironment::systemEnvironment();
    env.insert("BALANCE_ATTEST_REPO", repoDir());
    env.insert("LOGOS_BALANCE_ATTESTATION_ROOT", repoDir());
    env.insert("LOGOS_LEZ_REPO", lezRepoDir());
    env.insert("LEZ_REPO", lezRepoDir());
    env.insert("NSSA_WALLET_HOME_DIR", walletHomeDir());
    env.insert("RISC0_DEV_MODE", realProving() ? "0" : "1");
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
    setStatus(program + " " + arguments.join(" "));

    auto *process = new QProcess(this);
    process->setWorkingDirectory(repoDir());
    process->setProcessEnvironment(processEnvironment(envOverrides));

    connect(process, QOverload<int, QProcess::ExitStatus>::of(&QProcess::finished), this, [this, process, outputTarget](int exitCode, QProcess::ExitStatus exitStatus) {
        const auto stdoutText = QString::fromUtf8(process->readAllStandardOutput());
        const auto stderrText = QString::fromUtf8(process->readAllStandardError());
        const auto combined = QString(stdoutText + (stderrText.isEmpty() ? QString() : "\n" + stderrText)).trimmed();

        if (exitStatus == QProcess::NormalExit && exitCode == 0) {
            if (outputTarget == OutputTarget::ProofRun) {
                setProofRunJson(readTextFile(proofRunDir() + "/run.json"));
                setVerifyJson(readTextFile(proofRunDir() + "/verify.json"));
            } else if (outputTarget == OutputTarget::Verify) {
                setVerifyJson(stdoutText.trimmed());
            } else if (outputTarget == OutputTarget::GateRun) {
                setGateRunJson(readTextFile(gateRunDir() + "/run.json"));
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

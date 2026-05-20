#pragma once

#include <QMap>
#include <QProcess>
#include <QProcessEnvironment>
#include <QString>

#include "rep_balance_attestation_source.h"

class BalanceAttestationBackend : public BalanceAttestationSimpleSource {
    Q_OBJECT

public:
    explicit BalanceAttestationBackend(QObject *parent = nullptr);

    void setRepoDir(QString repoDir) override;
    void setLezRepoDir(QString lezRepoDir) override;
    void setWalletHomeDir(QString walletHomeDir) override;
    void setPrivateAccount(QString privateAccount) override;
    void setThreshold(QString threshold) override;
    void setChainIdHex(QString chainIdHex) override;
    void setVerifierIdHex(QString verifierIdHex) override;
    void setGateIdHex(QString gateIdHex) override;
    void setPresentationChallengeHex(QString presentationChallengeHex) override;
    void setRealProving(bool realProving) override;

public slots:
    void configureRepoDir(QString value) override;
    void configureLezRepoDir(QString value) override;
    void configureWalletHomeDir(QString value) override;
    void configurePrivateAccount(QString value) override;
    void configureThreshold(QString value) override;
    void configureChainIdHex(QString value) override;
    void configureVerifierIdHex(QString value) override;
    void configureGateIdHex(QString value) override;
    void configurePresentationChallengeHex(QString value) override;
    void configureRealProving(bool value) override;

    void runPreflight() override;
    void generateProof() override;
    void verifyEnvelope() override;
    void executeGateAdmit() override;
    void clearOutputs() override;

protected:
    void setBusy(bool value) override;
    void setStatus(QString value) override;
    void setProofRunDir(QString value) override;
    void setGateRunDir(QString value) override;
    void setProofRunJson(QString value) override;
    void setVerifyJson(QString value) override;
    void setGateRunJson(QString value) override;

private:
    enum class OutputTarget {
        Status,
        ProofRun,
        Verify,
        GateRun,
    };

    bool validateCommonInputs(bool requireProofRun, bool requireWalletAccount = false);
    bool validatePrivateAccountInWallet();
    QString normalizedPrivateAccount() const;
    QString proofDemoDir() const;
    QString gateDemoDir() const;
    QString scriptPath(const QString &name) const;
    QString readTextFile(const QString &path) const;
    QString timestamp() const;

    QProcessEnvironment processEnvironment(const QMap<QString, QString> &overrides = {}) const;
    void runProcess(
        const QString &program,
        const QStringList &arguments,
        const QMap<QString, QString> &envOverrides,
        OutputTarget outputTarget
    );
};

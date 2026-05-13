#pragma once

#include <QObject>
#include <QMap>
#include <QProcess>
#include <QProcessEnvironment>
#include <QString>

class BalanceAttestationBackend : public QObject {
    Q_OBJECT
    Q_PROPERTY(QString repoDir READ repoDir WRITE configureRepoDir NOTIFY repoDirChanged)
    Q_PROPERTY(QString lezRepoDir READ lezRepoDir WRITE configureLezRepoDir NOTIFY lezRepoDirChanged)
    Q_PROPERTY(QString walletHomeDir READ walletHomeDir WRITE configureWalletHomeDir NOTIFY walletHomeDirChanged)
    Q_PROPERTY(QString privateAccount READ privateAccount WRITE configurePrivateAccount NOTIFY privateAccountChanged)
    Q_PROPERTY(QString threshold READ threshold WRITE configureThreshold NOTIFY thresholdChanged)
    Q_PROPERTY(QString chainIdHex READ chainIdHex WRITE configureChainIdHex NOTIFY chainIdHexChanged)
    Q_PROPERTY(QString verifierIdHex READ verifierIdHex WRITE configureVerifierIdHex NOTIFY verifierIdHexChanged)
    Q_PROPERTY(QString gateIdHex READ gateIdHex WRITE configureGateIdHex NOTIFY gateIdHexChanged)
    Q_PROPERTY(QString presentationChallengeHex READ presentationChallengeHex WRITE configurePresentationChallengeHex NOTIFY presentationChallengeHexChanged)
    Q_PROPERTY(bool realProving READ realProving WRITE configureRealProving NOTIFY realProvingChanged)
    Q_PROPERTY(bool busy READ busy NOTIFY busyChanged)
    Q_PROPERTY(QString status READ status NOTIFY statusChanged)
    Q_PROPERTY(QString proofRunDir READ proofRunDir NOTIFY proofRunDirChanged)
    Q_PROPERTY(QString gateRunDir READ gateRunDir NOTIFY gateRunDirChanged)
    Q_PROPERTY(QString proofRunJson READ proofRunJson NOTIFY proofRunJsonChanged)
    Q_PROPERTY(QString verifyJson READ verifyJson NOTIFY verifyJsonChanged)
    Q_PROPERTY(QString gateRunJson READ gateRunJson NOTIFY gateRunJsonChanged)

public:
    explicit BalanceAttestationBackend(QObject *parent = nullptr);

    QString repoDir() const;
    QString lezRepoDir() const;
    QString walletHomeDir() const;
    QString privateAccount() const;
    QString threshold() const;
    QString chainIdHex() const;
    QString verifierIdHex() const;
    QString gateIdHex() const;
    QString presentationChallengeHex() const;
    bool realProving() const;
    bool busy() const;
    QString status() const;
    QString proofRunDir() const;
    QString gateRunDir() const;
    QString proofRunJson() const;
    QString verifyJson() const;
    QString gateRunJson() const;

    Q_INVOKABLE void configureRepoDir(const QString &value);
    Q_INVOKABLE void configureLezRepoDir(const QString &value);
    Q_INVOKABLE void configureWalletHomeDir(const QString &value);
    Q_INVOKABLE void configurePrivateAccount(const QString &value);
    Q_INVOKABLE void configureThreshold(const QString &value);
    Q_INVOKABLE void configureChainIdHex(const QString &value);
    Q_INVOKABLE void configureVerifierIdHex(const QString &value);
    Q_INVOKABLE void configureGateIdHex(const QString &value);
    Q_INVOKABLE void configurePresentationChallengeHex(const QString &value);
    Q_INVOKABLE void configureRealProving(bool value);

    Q_INVOKABLE void runPreflight();
    Q_INVOKABLE void generateProof();
    Q_INVOKABLE void verifyEnvelope();
    Q_INVOKABLE void executeGateAdmit();
    Q_INVOKABLE void clearOutputs();

signals:
    void repoDirChanged();
    void lezRepoDirChanged();
    void walletHomeDirChanged();
    void privateAccountChanged();
    void thresholdChanged();
    void chainIdHexChanged();
    void verifierIdHexChanged();
    void gateIdHexChanged();
    void presentationChallengeHexChanged();
    void realProvingChanged();
    void busyChanged();
    void statusChanged();
    void proofRunDirChanged();
    void gateRunDirChanged();
    void proofRunJsonChanged();
    void verifyJsonChanged();
    void gateRunJsonChanged();

private:
    enum class OutputTarget {
        Status,
        ProofRun,
        Verify,
        GateRun,
    };

    void setBusy(bool value);
    void setStatus(const QString &value);
    void setProofRunDir(const QString &value);
    void setGateRunDir(const QString &value);
    void setProofRunJson(const QString &value);
    void setVerifyJson(const QString &value);
    void setGateRunJson(const QString &value);

    bool validateCommonInputs(bool requireProofRun);
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

    QString m_repoDir;
    QString m_lezRepoDir;
    QString m_walletHomeDir;
    QString m_privateAccount;
    QString m_threshold = "1";
    QString m_chainIdHex;
    QString m_verifierIdHex;
    QString m_gateIdHex;
    QString m_presentationChallengeHex;
    bool m_realProving = true;
    bool m_busy = false;
    QString m_status = "Ready";
    QString m_proofRunDir;
    QString m_gateRunDir;
    QString m_proofRunJson;
    QString m_verifyJson;
    QString m_gateRunJson;
};

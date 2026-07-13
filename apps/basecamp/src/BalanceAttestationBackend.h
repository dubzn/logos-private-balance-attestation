#pragma once

#include <QByteArray>
#include <QList>
#include <QMap>
#include <QProcess>
#include <QProcessEnvironment>
#include <QString>

#include "rep_balance_attestation_source.h"

class LogosAPI;
class LogosModules;

class BalanceAttestationBackend : public BalanceAttestationSimpleSource {
    Q_OBJECT

public:
    explicit BalanceAttestationBackend(QObject *parent = nullptr);
    ~BalanceAttestationBackend() override;

    void initializeLogos(LogosAPI *api);

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
    void setDeliveryPreset(QString deliveryPreset) override;
    void setDeliveryMode(QString deliveryMode) override;
    void setDeliveryTopic(QString deliveryTopic) override;
    void setDeliveryGroupId(QString deliveryGroupId) override;
    void setDeliverySender(QString deliverySender) override;
    void setDeliveryRecipient(QString deliveryRecipient) override;

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
    void configureDeliveryPreset(QString value) override;
    void configureDeliveryMode(QString value) override;
    void configureDeliveryTopic(QString value) override;
    void configureDeliveryGroupId(QString value) override;
    void configureDeliverySender(QString value) override;
    void configureDeliveryRecipient(QString value) override;

    void runPreflight() override;
    void generateProof() override;
    void verifyEnvelope() override;
    void executeGateAdmit() override;
    void deliveryCreateNode() override;
    void deliverySubscribe() override;
    void deliverySendProofMessage() override;
    void deliveryVerifyReceivedMessage() override;
    void clearOutputs() override;
    void clearDelivery() override;

protected:
    void setBusy(bool value) override;
    void setStatus(QString value) override;
    void setProofRunDir(QString value) override;
    void setGateRunDir(QString value) override;
    void setProofRunJson(QString value) override;
    void setVerifyJson(QString value) override;
    void setGateRunJson(QString value) override;
    void setDeliveryStatus(QString value) override;
    void setDeliveryPeerId(QString value) override;
    void setDeliveryVersion(QString value) override;
    void setDeliveryNodeStarted(bool value) override;
    void setDeliverySubscribed(bool value) override;
    void setDeliveryReceived(bool value) override;
    void setDeliveryRunDir(QString value) override;
    void setDeliveryMessageJson(QString value) override;
    void setDeliveryVerifyJson(QString value) override;
    void setDeliveryLog(QString value) override;

private:
    enum class OutputTarget {
        Status,
        ProofRun,
        Verify,
        GateRun,
        DeliveryVerify,
    };

    bool validateCommonInputs(bool requireProofRun, bool requireWalletAccount = false);
    bool validatePrivateAccountInWallet();
    QString normalizedPrivateAccount() const;
    QString proofDemoDir() const;
    QString gateDemoDir() const;
    QString deliveryDemoDir() const;
    QString scriptPath(const QString &name) const;
    QString readTextFile(const QString &path) const;
    QString timestamp() const;
    QString deliveryMessagePath() const;
    QString deliveryGatePath() const;
    bool writeDeliveryGateFile(const QString &path);
    bool writeDeliveryMessageFile(const QString &path, QString *messageJson);
    bool ensureDeliveryReady(bool requireProofRun = false);
    void handleDeliveryPayload(const QString &topic, const QByteArray &payload, const QString &messageHash, qint64 timestampNs);
    void handleDeliveryChunk(const QString &topic, const QJsonObject &chunk, const QString &messageHash, qint64 timestampNs);
    void persistReceivedDeliveryMessage(const QString &topic, const QByteArray &payload, const QString &messageHash, qint64 timestampNs);
    void sendNextDeliveryChunk();
    void clearPendingDeliverySend();
    void appendDeliveryLog(const QString &line);
    void wireDeliveryEvents();
    void refreshDeliveryPeerId();

    QProcessEnvironment processEnvironment(const QMap<QString, QString> &overrides = {}) const;
    void runProcess(
        const QString &program,
        const QStringList &arguments,
        const QMap<QString, QString> &envOverrides,
        OutputTarget outputTarget
    );

    LogosModules *m_logos = nullptr;
    bool m_deliveryNodeStarted = false;
    bool m_deliverySubscribed = false;
    QMap<QString, QMap<int, QByteArray>> m_deliveryChunks;
    QMap<QString, int> m_deliveryChunkTotals;
    QMap<QString, QString> m_deliveryChunkSha256;
    QList<QByteArray> m_deliveryPendingChunks;
    QString m_deliveryPendingTopic;
    QString m_deliveryPendingMessageId;
    QString m_deliveryPendingPayloadHash;
    int m_deliveryPendingChunkIndex = 0;
};

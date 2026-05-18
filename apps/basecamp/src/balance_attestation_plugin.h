#pragma once

#include <QObject>

#include "LogosViewPluginBase.h"
#include "balance_attestation_interface.h"

class BalanceAttestationBackend;
class LogosAPI;

class BalanceAttestationPlugin : public QObject,
                                 public BalanceAttestationInterface,
                                 public BalanceAttestationViewPluginBase {
    Q_OBJECT
    Q_PLUGIN_METADATA(IID BalanceAttestationInterface_iid FILE "metadata.json")
    Q_INTERFACES(BalanceAttestationInterface)

public:
    explicit BalanceAttestationPlugin(QObject *parent = nullptr);
    ~BalanceAttestationPlugin() override;

    QString name() const override { return "balance_attestation"; }
    QString version() const override { return "0.1.0"; }

    Q_INVOKABLE void initLogos(LogosAPI *api);

private:
    BalanceAttestationBackend *m_backend = nullptr;
};

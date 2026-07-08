#include "balance_attestation_plugin.h"

#include "BalanceAttestationBackend.h"

#include <QDebug>

BalanceAttestationPlugin::BalanceAttestationPlugin(QObject *parent)
    : QObject(parent)
{
}

BalanceAttestationPlugin::~BalanceAttestationPlugin() = default;

void BalanceAttestationPlugin::initLogos(LogosAPI *api)
{
    if (m_backend) {
        return;
    }
    m_backend = new BalanceAttestationBackend(this);
    m_backend->initializeLogos(api);
    setBackend(m_backend);
    qDebug() << "BalanceAttestationPlugin: backend initialized";
}

#pragma once

#include "interface.h"

class BalanceAttestationInterface : public PluginInterface {
public:
    ~BalanceAttestationInterface() override = default;
};

#define BalanceAttestationInterface_iid "org.logos.BalanceAttestationInterface"
Q_DECLARE_INTERFACE(BalanceAttestationInterface, BalanceAttestationInterface_iid)

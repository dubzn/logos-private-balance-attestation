cmake_minimum_required(VERSION 3.14)
project(balance_attestation LANGUAGES CXX)

if(DEFINED ENV{LOGOS_MODULE_BUILDER_ROOT})
    include($ENV{LOGOS_MODULE_BUILDER_ROOT}/cmake/LogosModule.cmake)
else()
    message(FATAL_ERROR "LogosModule.cmake not found. Set LOGOS_MODULE_BUILDER_ROOT.")
endif()

logos_module(
    NAME balance_attestation
    REP_FILE src/balance_attestation.rep
    SOURCES
        src/BalanceAttestationBackend.cpp
        src/BalanceAttestationBackend.h
        src/balance_attestation_interface.h
        src/balance_attestation_plugin.cpp
        src/balance_attestation_plugin.h
)

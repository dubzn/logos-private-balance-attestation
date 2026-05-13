cmake_minimum_required(VERSION 3.22)
project(balance_attestation LANGUAGES CXX)

set(CMAKE_CXX_STANDARD 20)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_AUTOMOC ON)

find_package(Qt6 REQUIRED COMPONENTS Core Qml RemoteObjects)

qt_add_library(balance_attestation_plugin SHARED
    src/BalanceAttestationBackend.cpp
    src/BalanceAttestationBackend.h
)

target_link_libraries(balance_attestation_plugin
    PRIVATE
        Qt6::Core
        Qt6::Qml
        Qt6::RemoteObjects
)

target_include_directories(balance_attestation_plugin PRIVATE src)

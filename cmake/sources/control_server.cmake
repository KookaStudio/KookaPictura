# control_server sources for pictura_shell.
# Included from the root CMakeLists.txt; keep paths explicit (no globbing).
list(APPEND PICTURA_SHELL_SOURCES
    crates/pictura-app/cpp/agent_control.cpp
    crates/pictura-app/cpp/agent_control.h
    crates/pictura-app/cpp/client_registration.cpp
    crates/pictura-app/cpp/client_registration.h
    crates/pictura-app/cpp/client_registration_box.cpp
    crates/pictura-app/cpp/client_registration_box.h
    crates/pictura-app/cpp/control_approval_bar.cpp
    crates/pictura-app/cpp/control_approval_bar.h
    crates/pictura-app/cpp/control_policy.cpp
    crates/pictura-app/cpp/control_policy.h
    crates/pictura-app/cpp/control_scope.cpp
    crates/pictura-app/cpp/control_scope.h
    crates/pictura-app/cpp/control_server.cpp
    crates/pictura-app/cpp/control_server.h
    crates/pictura-app/cpp/control_server_actions.cpp
    crates/pictura-app/cpp/control_server_capture.cpp
    crates/pictura-app/cpp/control_server_commands.cpp
    crates/pictura-app/cpp/control_server_documents.cpp
    crates/pictura-app/cpp/control_server_gate.cpp
    crates/pictura-app/cpp/control_server_input.cpp
    crates/pictura-app/cpp/control_server_support.h
    crates/pictura-app/cpp/control_server_type.cpp
)
